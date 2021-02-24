#![allow(clippy::useless_vec)]
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
    time::Instant,
};

use self::error::Bf4Result;
use crate::rcon::{err_none, ok_eof, packet::Packet, RconClient, RconError, RconResult, RconQueryable};
use ascii::{AsciiStr, AsciiString, IntoAsciiString};
use error::Bf4Error;
use futures_core::Stream;
use player_info_block::PlayerInfo;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

pub mod defs;
pub mod ea_guid;
pub mod error;
pub mod map_list;
pub mod player_info_block;
mod util;

pub use defs::{Event, GameMode, Map, Player, Squad, Team, Visibility, Weapon};
pub use ea_guid::Eaid;

// cmd_err!(pub PlayerKickError, PlayerNotFound, A);
cmd_err!(pub PlayerKillError, InvalidPlayerName, SoldierNotAlive);
cmd_err!(pub SayError, MessageTooLong, PlayerNotFound);
cmd_err!(pub ListPlayersError, );
cmd_err!(pub MapListError, MapListFull, InvalidGameMode, InvalidMapIndex, InvalidRoundsPerMap);

pub(crate) trait RconEncoding: Sized {
    fn rcon_encode(&self) -> AsciiString;
    fn rcon_decode(ascii: &AsciiStr) -> RconResult<Self>;
}

#[derive(Debug, Copy, Clone)]
struct PlayerCacheEntry {
    freshness: Instant,
    eaid: Eaid,
}

/// You should never need to worry about the `F` generic parameter.
/// It should be automatically inferred from the event handler you provide upon creation.
#[derive(Debug)]
pub struct Bf4Client {
    rcon: RconClient,
    /// You can `.subscribe()` to this and you'll received Events.
    events: broadcast::Sender<Bf4Result<Event>>,

    /// Needs to be in Bf4Client and behind a cache, since we'll be accessing this from two places:
    /// - When parsing packets, e.g. from events.
    /// - When parsing replies to queries.
    player_cache: Mutex<HashMap<AsciiString, PlayerCacheEntry>>,
    // /// Things that get periodically called.
    // interval_timers: Vec<Interval>,
}

impl Bf4Client {
    pub async fn new(mut rcon: RconClient) -> RconResult<Arc<Self>> {
        let (tx, rx) = oneshot::channel::<Weak<Bf4Client>>();

        let events = Bf4Client::packet_to_event_stream(rx, rcon.take_nonresponse_rx().expect("Bf4Client requires Rcon's `take_nonresponse_tx()` to succeed. If you are calling this yourself, then please don't."));
        let myself = Arc::new(Self {
            rcon,
            events,
            player_cache: Mutex::new(HashMap::new()),
            // interval_timers: Vec::new(),
        });

        tx.send(Arc::downgrade(&myself)).unwrap();

        myself.rcon.events_enabled(true).await?;

        Ok(myself)
    }

    pub async fn resolve_player(self: &Arc<Bf4Client>, name: &AsciiString) -> Bf4Result<Player> {
        let entry: Option<PlayerCacheEntry> = {
            let cache = self.player_cache.lock().await;
            cache.get(name).copied()
            // make sure we unlock the mutex quickly, especially since we might query.
        };

        if let Some(entry) = entry {
            // oh neat, player is already cached. No need for sending a command to rcon.
            // println!("[Bf4Client::resolve_player] Cache hit for {} -> {}", name, entry.eaid);
            Ok(Player {
                name: name.clone(),
                eaid: entry.eaid,
            })
        } else {
            // welp, gotta ask rcon and update cache...
            // println!("[Bf4Client::resolve_player] Cache miss for {}, resolving...", name);
            let mut pib = match self.list_players(Visibility::All).await {
                // hm, sucks that you need clone for this :/
                Ok(pib) => pib,
                Err(ListPlayersError::Rcon(rcon)) => {
                    return Err(Bf4Error::PlayerGuidResolveFailed {
                        player_name: name.clone(),
                        rcon: Some(rcon),
                    });
                }
            };

            let mut cache = self.player_cache.lock().await;
            // technically it's possible someone else updated the cache meanwhile, but that's fine.
            for pi in &mut pib {
                cache.insert(
                    pi.player_name.clone(),
                    PlayerCacheEntry {
                        freshness: Instant::now(),
                        eaid: pi.eaid,
                    },
                );

                // println!("Resolved");
            }

            match pib.iter().find(|pi| &pi.player_name == name) {
                Some(pi) => {
                    let player = Player {
                        name: pi.player_name.clone(),
                        eaid: pi.eaid,
                    };
                    Ok(player)
                }
                None => Err(Bf4Error::PlayerGuidResolveFailed {
                    player_name: name.clone(),
                    rcon: None,
                }),
            }
        }
    }

    async fn parse_packet(bf4client: &Weak<Bf4Client>, packet: Packet) -> Bf4Result<Event> {
        // helper function
        fn upgrade(bf4client: &Weak<Bf4Client>) -> Bf4Result<Arc<Bf4Client>> {
            match bf4client.upgrade() {
                Some(arc) => Ok(arc),
                None => Err(Bf4Error::other(
                    "[Bf4Client::parse_packet] Bf4Client is already dropped.",
                )),
            }
        }

        fn assert_len(packet: &Packet, n: usize) -> Bf4Result<()> {
            if packet.words.len() != n {
                Err(Bf4Error::Rcon(RconError::malformed_packet(
                    packet.words.clone(),
                    format!("{} packet must have {} words", &packet.words[0], n),
                )))
            } else {
                Ok(())
            }
        }

        match packet.words[0].as_str() {
            "player.onKill" => {
                assert_len(&packet, 5)?;
                let bf4 = upgrade(bf4client)?;
                Ok(Event::Kill {
                    killer: if packet.words[1].is_empty() {
                        None
                    } else {
                        Some(bf4.resolve_player(&packet.words[1]).await?)
                    },
                    victim: bf4.resolve_player(&packet.words[2]).await?,
                    weapon: Weapon::Other(packet.words[3].clone()),
                    headshot: false,
                })
            }
            "player.onSpawn" => {
                assert_len(&packet, 3)?;
                let bf4 = upgrade(bf4client)?;
                Ok(Event::Spawn {
                    player: bf4.resolve_player(&packet.words[1]).await?,
                    team: Team::rcon_decode(&packet.words[2])?,
                })
            }
            "player.onChat" => {
                if packet.words.len() < 4 {
                    return Err(Bf4Error::Rcon(RconError::malformed_packet(
                        packet.words.clone(),
                        format!("{} packet must have at least {} words", &packet.words[0], 4),
                    )));
                }
                let bf4 = upgrade(bf4client)?;

                let (vis, consumed) = Visibility::rcon_decode(&packet.words[3..])?;
                if consumed + 3 == packet.words.len() {
                    let player_name = &packet.words[1];
                    if player_name == "Server" {
                        Ok(Event::ServerChat {
                            msg: packet.words[2].clone(),
                            vis,
                        })
                    } else {
                        Ok(Event::Chat {
                            player: bf4.resolve_player(&packet.words[1]).await?,
                            msg: packet.words[2].clone(),
                            vis,
                        })
                    }
                } else {
                    Err(Bf4Error::Rcon(RconError::malformed_packet(
                        packet.words,
                        "More words than expected",
                    )))
                }
            }
            "server.onRoundOver" => {
                if packet.words.len() != 2 {
                    return Err(Bf4Error::Rcon(RconError::malformed_packet(
                        packet.words.clone(),
                        format!("{} packet must have {} words", &packet.words[0], 2),
                    )));
                }
                Ok(Event::RoundOver {
                    winning_team: Team::rcon_decode(&packet.words[1])?,
                })
            }
            "punkBuster.onMessage" => {
                assert_len(&packet, 2)?;
                Ok(Event::PunkBusterMessage(packet.words[1].to_string()))
            }
            _ => Err(Bf4Error::UnknownEvent(packet.words)),
        }
    }

    /// Takes packets, transforms into events, broadcasts.
    fn packet_to_event_stream(
        bf4client_rx: oneshot::Receiver<Weak<Bf4Client>>,
        mut packets: mpsc::UnboundedReceiver<RconResult<Packet>>,
    ) -> broadcast::Sender<Bf4Result<Event>> {
        let (tx, _) = broadcast::channel::<Bf4Result<Event>>(128);
        let tx2 = tx.clone();
        tokio::spawn(async move {
            let bf4client: Weak<Bf4Client> = bf4client_rx.await.unwrap();
            while let Some(packet) = packets.recv().await {
                // println!("[Bf4Clinet::packet_to_event_stream] Received {:?}", packet);
                match packet {
                    Ok(packet) => {
                        if packet.words.is_empty() {
                            let _ = tx2.send(Err(RconError::protocol_msg(
                                "Received empty packet somehow?",
                            )
                            .into())); // All events must have at least one word.
                            continue; // should probably be a break, but yeah whatever.
                        }
                        let event = Bf4Client::parse_packet(&bf4client, packet).await;
                        let _ = tx2.send(event); // actually broadcast packet to all event streams.
                    }
                    Err(e) => {
                        // the packet receiver loop (tcp,...) encountered an error. This will most of the time
                        // be something such as connection dropped.
                        let _ = tx2.send(Err(e.into()));
                        continue; // should probably a break, but yeah whatever.
                    }
                }
            }
            // println!("[Bf4Client::packet_to_event_stream] Ended");
        });

        tx
    }

    fn event_stream_raw(&self) -> BroadcastStream<Bf4Result<Event>> {
        let rx = self.events.subscribe();
        tokio_stream::wrappers::BroadcastStream::new(rx)
    }

    /// This function differs from the raw version by unpacking potential broadcast errors,
    /// which only occur when the backlog of unhandled events gets too big (128 currently).
    /// In that case, those events are simply ignored and only a warning is emitted.
    /// If you want to handle the overflow error yourself, use `event_stream_raw`.
    pub fn event_stream(&self) -> impl Stream<Item = Bf4Result<Event>> {
        self.event_stream_raw().filter_map(|ev| {
            match ev {
                Ok(x) => Some(x),
                Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
                    println!("warn [Bf4Client::event_stream] Too many events at once! Had to drop {} events", n);
                    None // filter out errors like this.
                },
            }
        })
    }

    pub async fn list_players(&self, vis: Visibility) -> Result<Vec<PlayerInfo>, ListPlayersError> {
        let mut words = veca!["admin.listPlayers"];
        words.append(&mut vis.rcon_encode());
        self.rcon
            .query(
                &words,
                |ok| player_info_block::parse_pib(&ok).map_err(|rconerr| rconerr.into()),
                err_none,
            )
            .await
    }

    pub async fn kill(
        &self,
        player: impl IntoAsciiString + Into<String>,
    ) -> Result<(), PlayerKillError> {
        // first, `command` checks whether we received an OK, if yes, calls `ok`.
        // if not, then it checks if the response was `UnknownCommand` or `InvalidArguments`,
        // and handles those with an appropriate error message.
        // if not, then it calls `err`, hoping for a `Some(err)`, but if it returns `None`,
        // then it just creates an `RconError::UnknownResponse` error.
        let player = player.into_ascii_string()?;
        self.rcon
            .query(
                &veca!["admin.killPlayer", player],
                ok_eof,
                |err| match err {
                    "InvalidPlayerName" => Some(PlayerKillError::InvalidPlayerName),
                    "SoldierNotAlive" => Some(PlayerKillError::SoldierNotAlive),
                    _ => None,
                },
            )
            .await
    }

    pub async fn say(
        &self,
        msg: impl IntoAsciiString + Into<String>,
        vis: Visibility,
    ) -> Result<(), SayError> {
        let mut words = veca!["admin.say", msg];
        words.append(&mut vis.rcon_encode());
        self.rcon
            .query(&words, ok_eof, |err| match err {
                "InvalidTeam" => Some(SayError::Rcon(RconError::protocol_msg(
                    "Rcon did not understand our teamId",
                ))),
                "InvalidSquad" => Some(SayError::Rcon(RconError::protocol_msg(
                    "Rcon did not understand our squadId",
                ))),
                "MessageTooLong" => Some(SayError::MessageTooLong),
                "PlayerNotFound" => Some(SayError::PlayerNotFound),
                _ => None,
            })
            .await
    }



    /// Prints multiple lines at once.
    /// Sends all `say` commands first, each in a `tokio::spawn(..)`, then waits until
    /// they all complete, returning the first `Err(..)` if any, otherwise `Ok(())`.
    ///
    /// Panics when joining a joinhandle fails, i.e. when the future itself panicked.
    /// You should never have to bother about this, if you encounter this, it's a bug.
    /// 
    /// # Other notes
    /// This function is fucking ugly internally...
    pub async fn say_lines<Line>(
        self: Arc<Self>,
        lines: impl IntoIterator<Item = Line>,
        vis: Visibility,
    ) -> Result<(), SayError>
        where Line: IntoAsciiString + Into<String> + 'static + Send,
    {
        let vis_words = vis.rcon_encode();
        let queries = lines.into_iter().map(|line| {
            let mut words = vec![
                "admin.say".into_ascii_string().unwrap(),
                line.into_ascii_string().unwrap(),
            ];
            words.append(&mut vis_words.clone());
            words
        }).collect::<Vec<_>>();

        match self.rcon.queries_raw(queries).await {
            Some(responses) => {
                for response in responses {
                    let response = response?;
                    match response[0].as_str() {
                        "OK" => {},
                        "InvalidTeam" => return Err(SayError::Rcon(RconError::protocol_msg(
                            "Rcon did not understand our teamId",
                        ))),
                        "InvalidSquad" => return Err(SayError::Rcon(RconError::protocol_msg(
                            "Rcon did not understand our squadId",
                        ))),
                        "MessageTooLong" => return Err(SayError::MessageTooLong),
                        "PlayerNotFound" => return Err(SayError::PlayerNotFound),
                        _ => return Err(SayError::Rcon(RconError::other("")))
                    }
                }
            }
            None => return Err(RconError::ConnectionClosed.into()),
        }

        Ok(())
    }

    pub async fn maplist_clear(&self) -> Result<(), MapListError> {
        self.rcon
            .query(&veca!["mapList.clear"], ok_eof, err_none)
            .await
    }

    pub async fn maplist_add(
        &self,
        map: Map,
        game_mode: GameMode,
        n_rounds: i32,
        offset: i32,
    ) -> Result<(), MapListError> {
        self.rcon
            .query(
                &veca![
                    "mapList.add",
                    map.rcon_encode(),
                    game_mode.rcon_encode(),
                    n_rounds.to_string(),
                    offset.to_string(),
                ],
                ok_eof,
                |err| match err {
                    "InvalidMap" => Some(MapListError::Rcon(RconError::protocol_msg(format!(
                        "Rcon did not understand our map name {}",
                        map.rcon_encode()
                    )))),
                    "Full" => Some(MapListError::MapListFull),
                    "InvalidGameModeOnMap" => Some(MapListError::InvalidGameMode), // chosen map + gamemode combo invalid (not always purely rcon error)
                    "InvalidRoundsPerMap" => Some(MapListError::InvalidRoundsPerMap),
                    "InvalidMapIndex" => Some(MapListError::InvalidMapIndex),
                    _ => None,
                },
            )
            .await
    }

    pub async fn maplist_list(&self) -> Result<Vec<map_list::MapListEntry>, MapListError> {
        self.rcon
            .query(
                &veca!["maplist.list", "0"],
                |ok| Ok(map_list::parse_map_list(&ok)?),
                err_none,
            )
            .await
    }

    pub async fn maplist_run_next_round(&self) -> Result<(), MapListError> {
        // TODO errors
        self.rcon
            .query(&veca!["mapList.runNextRound"], ok_eof, err_none)
            .await
    }
    pub async fn maplist_save(&self) -> Result<(), MapListError> {
        // TODO err
        self.rcon
            .query(&veca!["mapList.save"], ok_eof, err_none)
            .await
    }
    pub async fn maplist_restart_round(&self) -> Result<(), MapListError> {
        // TODO err
        self.rcon
            .query(&veca!["mapList.restartRound"], ok_eof, err_none)
            .await
    }
    pub async fn maplist_set_next_map(&self, index: usize) -> Result<(), MapListError> {
        // TODO err
        self.rcon
            .query(
                &veca![
                    "mapList.setNextMapIndex",
                    index.to_string().into_ascii_string().unwrap()
                ],
                ok_eof,
                err_none,
            )
            .await
    }
}

// impl Drop for Bf4Client {
//     fn drop(&mut self) {
//         println!("Dropped Bf4Client");
//     }
// }

#[cfg(test)]
mod test {
    use super::*;
    use crate::rcon;
    use std::time::Instant;

    async fn spammer(i: usize) -> rcon::RconResult<()> {
        let rcon = RconClient::connect(("127.0.0.1", 47200, "smurf")).await?;
        let bf4 = Bf4Client::new(rcon).await.unwrap();
        let start = Instant::now();

        let mut joinhandles = Vec::new();
        for _i in 0..10 {
            let bf4 = bf4.clone();
            joinhandles.push(tokio::spawn(async move { bf4.kill("player").await }));
        }

        for future in joinhandles {
            future.await.unwrap().unwrap_err();
        }

        // tokio::time::sleep(Duration::from_secs(2)).await;

        println!(
            "spammer#{}: Done receiving after {}ms",
            i,
            start.elapsed().as_millis()
        );
        Ok(())
    }

    #[tokio::test]
    // #[ignore]
    async fn spam() -> rcon::RconResult<()> {
        let mut joinhandles = Vec::new();
        for i in 0..10 {
            joinhandles.push(tokio::spawn(spammer(i)));
        }

        for future in joinhandles {
            future.await.unwrap().unwrap();
        }

        Ok(())
    }

    #[tokio::test]
    async fn lifetimes() -> RconResult<()> {
        let rcon = RconClient::connect(("127.0.0.1", 47200, "smurf")).await?;
        let bf4 = Bf4Client::new(rcon).await.unwrap();

        println!(
            "bf4 counts: {}, {}",
            Arc::strong_count(&bf4),
            Arc::weak_count(&bf4)
        );

        Ok(())
    }
}
