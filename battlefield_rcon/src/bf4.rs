#![allow(clippy::useless_vec)]
use std::sync::{Arc, Mutex, Weak};

use self::{defs::Preset, error::Bf4Result, player_cache::PlayerEaidCache};
use crate::rcon::{
    ok_eof, packet::Packet, RconClient, RconConnectionInfo, RconError, RconQueryable, RconResult,
};
use ascii::{AsciiStr, AsciiString, IntoAsciiString};
use error::Bf4Error;
use futures_core::Stream;
use player_info_block::{parse_pib, PlayerInfo};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

pub mod defs;
pub mod ea_guid;
pub mod error;
pub mod map_list;
pub(crate) mod player_cache;
pub mod player_info_block;
mod util;

pub use defs::{Event, GameMode, Map, Player, Squad, Team, Visibility, Weapon};
pub use ea_guid::Eaid;

// cmd_err!(pub PlayerKickError, PlayerNotFound, A);
cmd_err!(pub PlayerKillError, InvalidPlayerName, SoldierNotAlive);
cmd_err!(pub SayError, MessageTooLong, PlayerNotFound);
cmd_err!(pub ListPlayersError, );
cmd_err!(pub MapListError, MapListFull, InvalidGameMode, InvalidMapIndex, InvalidRoundsPerMap);
cmd_err!(pub ReservedSlotsError, PlayerAlreadyInList, ReservedSlotsFull);
cmd_err!(pub GameAdminError, Full, AlreadyInList);
cmd_err!(pub PlayerKickError, PlayerNotFound);

pub(crate) trait RconDecoding: Sized {
    fn rcon_decode(ascii: &AsciiStr) -> RconResult<Self>;
}

pub(crate) trait RconEncoding: RconDecoding {
    fn rcon_encode(&self) -> AsciiString;
}

/// You should never need to worry about the `F` generic parameter.
/// It should be automatically inferred from the event handler you provide upon creation.
#[derive(Debug)]
pub struct Bf4Client {
    rcon: RconClient,
    /// You can `.subscribe()` to this and you'll received Events.
    /// The mutex is here only as a thread-safe `Cell<T>`.
    events: Mutex<Option<broadcast::Sender<Bf4Result<Event>>>>,

    /// Needs to be in Bf4Client and behind a cache, since we'll be accessing this from two places:
    /// - When parsing packets, e.g. from events.
    /// - When parsing replies to queries.
    player_cache: Mutex<PlayerEaidCache>,
}

impl Bf4Client {
    pub async fn connect(coninfo: &RconConnectionInfo) -> RconResult<Arc<Self>> {
        let rcon = RconClient::connect(coninfo).await?;
        Bf4Client::new_from(rcon).await
    }

    pub async fn new_from(mut rcon: RconClient) -> RconResult<Arc<Self>> {
        let (tx, rx) = oneshot::channel::<Weak<Bf4Client>>();

        let events = Bf4Client::packet_to_event_stream(rx, rcon.take_nonresponse_rx().expect("Bf4Client requires Rcon's `take_nonresponse_tx()` to succeed. If you are calling this yourself, then please don't."));
        let myself = Arc::new(Self {
            rcon,
            events: Mutex::new(Some(events)),
            player_cache: Mutex::new(PlayerEaidCache::new()),
            // interval_timers: Vec::new(),
        });

        tx.send(Arc::downgrade(&myself)).unwrap();

        Ok(myself)
    }

    pub async fn resolve_player(&self, name: &AsciiString) -> Bf4Result<Player> {
        let entry = {
            let mut cache = self
                .player_cache
                .lock()
                .expect("Could not lock player cache, it is poisoned");
            cache.try_get(name)
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

            let mut cache = self
                .player_cache
                .lock()
                .expect("Failed to acquire mutex lock on player cache");
            // technically it's possible someone else updated the cache meanwhile, but that's fine.
            for pi in &mut pib {
                cache.insert(&pi.player_name, &pi.eaid);
            }
            drop(cache);

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

    // Set a player's GUID. For example on join this is used.
    pub fn player_has_guid(&self, name: &AsciiString, eaid: &Eaid) {
        let mut lock = self
            .player_cache
            .lock()
            .expect("Failed to acquire mutex lock on player cache");
        lock.insert(name, eaid);
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
                let killer = if packet.words[1].is_empty() {
                    None
                } else {
                    Some(bf4.resolve_player(&packet.words[1]).await?)
                };
                Ok(Event::Kill {
                    killer,
                    victim: bf4.resolve_player(&packet.words[2]).await?,
                    weapon: Weapon::rcon_decode(&packet.words[3])?,
                    // weapon: Weapon::Other(packet.words[3].clone()),
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
            "player.onSquadChange" => {
                if packet.words.len() != 4 {
                    return Err(Bf4Error::Rcon(RconError::malformed_packet(
                        packet.words.clone(),
                        format!("{} packet must have {} words", &packet.words[0], 4),
                    )));
                }
                let bf4 = upgrade(bf4client)?;
                Ok(Event::SquadChange {
                    player: bf4.resolve_player(&packet.words[1]).await?,
                    team: Team::rcon_decode(&packet.words[2])?,
                    squad: Squad::rcon_decode(&packet.words[3])?,
                })
            }
            "player.onTeamChange" => {
                if packet.words.len() != 4 {
                    return Err(Bf4Error::Rcon(RconError::malformed_packet(
                        packet.words.clone(),
                        format!("{} packet must have {} words", &packet.words[0], 4),
                    )));
                }
                let bf4 = upgrade(bf4client)?;
                Ok(Event::TeamChange {
                    player: bf4.resolve_player(&packet.words[1]).await?,
                    team: Team::rcon_decode(&packet.words[2])?,
                    squad: Squad::rcon_decode(&packet.words[3])?,
                })
            }
            "player.onJoin" => {
                if packet.words.len() != 3 {
                    return Err(Bf4Error::Rcon(RconError::malformed_packet(
                        packet.words.clone(),
                        format!("{} packet must have {} words", &packet.words[0], 3),
                    )));
                }
                let eaid = Eaid::from_rcon_format(&packet.words[2]).map_err(|_| {
                    RconError::malformed_packet(
                        packet.words.clone(),
                        format!(
                            "Failed to parse the following as an EAID: {}",
                            packet.words[2]
                        ),
                    )
                })?;
                let bf4 = upgrade(bf4client)?;
                bf4.player_has_guid(&packet.words[1], &eaid); // while we have the GUID, might as well notify the cache about it.
                Ok(Event::Join {
                    player: Player {
                        name: packet.words[1].clone(),
                        eaid,
                    },
                })
            }
            "player.onAuthenticated" => {
                if packet.words.len() != 2 {
                    return Err(Bf4Error::Rcon(RconError::malformed_packet(
                        packet.words.clone(),
                        format!("{} packet must have {} words", &packet.words[0], 2),
                    )));
                }
                let bf4 = upgrade(bf4client)?;
                Ok(Event::Authenticated {
                    player: bf4.resolve_player(&packet.words[1]).await?,
                    // TODO maybe use the GUID from here directly instead of resolving, but oh well...
                })
            }
            "player.onLeave" => {
                // TODO check param count
                let player_name = &packet.words[1];
                let pib = parse_pib(&packet.words[2..])?;
                if pib.len() == 1 {
                    if &pib[0].player_name == player_name {
                        Ok(Event::Leave {
                            player: Player {
                                name: player_name.to_owned(),
                                eaid: pib[0].eaid,
                            },
                            final_scores: pib[0].to_owned(),
                        })
                    } else {
                        Err(Bf4Error::Rcon(RconError::malformed_packet(
                            packet.words.clone(),
                            format!(
                                "Somehow {}'s onLeave event contained a PlayerInfo for {}? Wtf?",
                                player_name, pib[0].player_name
                            ),
                        )))
                    }
                } else {
                    Err(Bf4Error::Rcon(RconError::malformed_packet(packet.words.clone(), format!("Expected exactly one PlayerInfo entry for onLeave packet, but found {} entries instead", pib.len()))))
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
            // Just for initialization: bf4client constructor sends us an instance of itself.
            // Has to be like this since we can't pass an instance via parameter here.
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
            if let Some(bf4client) = bf4client.upgrade() {
                // that's it, no more events.
                // close all the event_streams, make them return `None` and end the while let loop.
                bf4client.drop_events_sender();
            } else {
                // bf4client is already dropped
                // ==> `events` inside it is already dropped
                // ==> that's what we wanted anyway, so... nothing to do :).
            }
            // println!("[Bf4Client::packet_to_event_stream] Ended");
        });

        tx
    }

    /// Drops events sender, causing all event_streams to end.
    fn drop_events_sender(&self) {
        let mut lock = self
            .events
            .lock()
            .expect("Failed to acquire mutex lock Bf4Client::events");
        lock.take();
    }

    /// Errors:
    /// When the underlying events stream has already ended, returns `Err(())`
    async fn event_stream_raw(&self) -> RconResult<BroadcastStream<Bf4Result<Event>>> {
        self.rcon.events_enabled(true).await?;

        let lock = self
            .events
            .lock()
            .expect("Failed to acquire mutex lock Bf4Client::events");
        if let Some(events) = lock.as_ref() {
            let rx = events.subscribe();
            Ok(tokio_stream::wrappers::BroadcastStream::new(rx))
        } else {
            // create a dummy empty stream which immediately closes
            let (_, rx) = tokio::sync::broadcast::channel(1);
            Ok(tokio_stream::wrappers::BroadcastStream::new(rx))
        }
    }

    /// This function differs from the raw version by unpacking potential broadcast errors,
    /// which only occur when the backlog of unhandled events gets too big (128 currently).
    /// In that case, those events are simply ignored and only a warning is emitted.
    /// If you want to handle the overflow error yourself, use `event_stream_raw`.
    pub async fn event_stream(&self) -> RconResult<impl Stream<Item = Bf4Result<Event>>> {
        Ok(self.event_stream_raw().await?.filter_map(|ev| {
            match ev {
                Ok(x) => Some(x),
                Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
                    println!("warn [Bf4Client::event_stream] Too many events at once! Had to drop {} events", n);
                    None // filter out errors like this.
                },
            }
        }))
    }

    pub async fn list_players(&self, vis: Visibility) -> Result<Vec<PlayerInfo>, ListPlayersError> {
        let mut words = veca!["admin.listPlayers"];
        words.append(&mut vis.rcon_encode());
        self.rcon
            .query(
                &words,
                |ok| parse_pib(&ok).map_err(|rconerr| rconerr.into()),
                |_| None,
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

    pub async fn kick(
        &self,
        player: impl IntoAsciiString + Into<String>,
        reason: impl IntoAsciiString + Into<String>,
    ) -> Result<(), PlayerKickError> {
        let player = player.into_ascii_string()?;
        let reason = reason.into_ascii_string()?;
        self.rcon
            .query(
                &veca!["admin.kickPlayer", player, reason],
                ok_eof,
                |err| match err {
                    "PlayerNotFound" => Some(PlayerKickError::PlayerNotFound),
                    _ => None,
                },
            )
            .await
    }

    pub async fn say(
        &self,
        msg: impl IntoAsciiString + Into<String>,
        vis: impl Into<Visibility>,
    ) -> Result<(), SayError> {
        let mut words = veca!["admin.say", msg];
        words.append(&mut vis.into().rcon_encode());
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
        self: &Arc<Self>,
        lines: impl IntoIterator<Item = Line>,
        vis: impl Into<Visibility>,
    ) -> Result<(), SayError>
    where
        Line: IntoAsciiString + Into<String> + 'static + Send,
    {
        let vis = vis.into();
        let vis_words = vis.rcon_encode();
        let queries = lines
            .into_iter()
            .map(|line| {
                let mut words = vec![
                    "admin.say".into_ascii_string().unwrap(),
                    line.into_ascii_string().unwrap(),
                ];
                words.append(&mut vis_words.clone());
                words
            })
            .collect::<Vec<_>>();

        match self.rcon.queries_raw(queries).await {
            Some(responses) => {
                for response in responses {
                    let response = response?;
                    match response[0].as_str() {
                        "OK" => {}
                        "InvalidTeam" => {
                            return Err(SayError::Rcon(RconError::protocol_msg(
                                "Rcon did not understand our teamId",
                            )))
                        }
                        "InvalidSquad" => {
                            return Err(SayError::Rcon(RconError::protocol_msg(
                                "Rcon did not understand our squadId",
                            )))
                        }
                        "MessageTooLong" => return Err(SayError::MessageTooLong),
                        "PlayerNotFound" => return Err(SayError::PlayerNotFound),
                        _ => return Err(SayError::Rcon(RconError::other(""))),
                    }
                }
            }
            None => return Err(RconError::ConnectionClosed.into()),
        }

        Ok(())
    }

    pub async fn maplist_clear(&self) -> Result<(), RconError> {
        self.rcon
            .query(&veca!["mapList.clear"], ok_eof, |_| None)
            .await
    }

    pub async fn maplist_add(
        &self,
        map: &Map,
        game_mode: &GameMode,
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
                |_| None,
            )
            .await
    }

    pub async fn maplist_run_next_round(&self) -> Result<(), MapListError> {
        // TODO errors
        self.rcon
            .query(&veca!["mapList.runNextRound"], ok_eof, |_| None)
            .await
    }
    pub async fn maplist_save(&self) -> Result<(), MapListError> {
        // TODO err
        self.rcon
            .query(&veca!["mapList.save"], ok_eof, |_| None)
            .await
    }
    pub async fn maplist_restart_round(&self) -> Result<(), MapListError> {
        // TODO err
        self.rcon
            .query(&veca!["mapList.restartRound"], ok_eof, |_| None)
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
                |_| None,
            )
            .await
    }
    pub async fn maplist_remove(&self, index: usize) -> Result<(), MapListError> {
        // TODO err
        self.rcon
            .query(
                &veca![
                    "mapList.remove",
                    index.to_string().into_ascii_string().unwrap()
                ],
                ok_eof,
                |_| None,
            )
            .await
    }

    /// # You probably shouldn't be using this unless you know what you're doing.
    pub fn get_underlying_rcon_client(&self) -> &RconClient {
        &self.rcon
    }

    pub async fn set_preset(&self, preset: Preset) -> RconResult<()> {
        self.rcon
            .query(
                &veca!["vars.preset", preset.rcon_encode(), "false"],
                ok_eof,
                |_| None,
            )
            .await
    }

    pub async fn set_tickets(&self, tickets: usize) -> RconResult<()> {
        self.rcon
            .query(
                &veca!["vars.gameModeCounter", format!("{}", tickets)],
                ok_eof,
                |_| None,
            )
            .await
    }

    pub async fn set_vehicles_spawn_allowed(&self, allowed: bool) -> RconResult<()> {
        self.rcon
            .query(
                &veca!["vars.vehicleSpawnAllowed", allowed.to_string()],
                ok_eof,
                |_| None,
            )
            .await
    }

    /// add player name to reserved slots list.
    pub async fn reserved_add(&self, player: &Player) -> Result<(), ReservedSlotsError> {
        self.rcon
            .query(
                &veca!["reservedSlotsList.add", player.name.as_str()],
                ok_eof,
                |err| match err {
                    "Full" => Some(ReservedSlotsError::ReservedSlotsFull),
                    "PlayerAlreadyInList" => Some(ReservedSlotsError::PlayerAlreadyInList),
                    _ => None,
                },
            )
            .await
    }

    /// Saves the reserved slots to file. (can fail, remote rcon io error, no error code yet.)
    pub async fn reserved_save(&self) -> Result<(), ReservedSlotsError> {
        // TODO err
        self.rcon
            .query(&veca!["reservedSlotsList.save"], ok_eof, |_| None)
            .await
    }

    /// Lists reserved slots + level
    pub async fn reserved_list(&self) -> Result<Vec<AsciiString>, RconError> {
        // TODO: technically only a limited amount of reserved slots are returned, and we need to
        // repeat the query with a different offset other than "0".
        self.rcon
            .query(
                &veca!["reservedSlotsList.list", "0"],
                |ok| Ok(ok.to_owned()),
                |_| None,
            )
            .await
    }

    pub async fn admin_add(&self, player: &Player, level: usize) -> Result<(), ReservedSlotsError> {
        self.rcon
            .query(
                &veca!["gameAdmin.add", player.name.as_str(), level.to_string()],
                ok_eof,
                |err| match err {
                    "Full" => Some(ReservedSlotsError::ReservedSlotsFull),
                    "PlayerAlreadyInList" => Some(ReservedSlotsError::PlayerAlreadyInList),
                    _ => None,
                },
            )
            .await
    }

    /// Lists the game Admins (undocumented in the PDF), together with their level (1, 2, 3).
    pub async fn admin_list(&self) -> Result<Vec<(AsciiString, usize)>, GameAdminError> {
        self.rcon
            .query(
                &veca!["gameAdmin.list", "0"],
                |ok| {
                    if ok.len() == 2 {
                        let mut vec = Vec::new();
                        let mut offset = 0;
                        while offset < ok.len() {
                            let level = ok[offset + 1].as_str().parse::<usize>().map_err(|_| {
                                RconError::protocol_msg("Expected int, received garbage.")
                            })?;
                            vec.push((ok[offset].clone(), level));
                            offset += 2;
                        }
                        Ok(vec)
                    } else {
                        Err(GameAdminError::Rcon(RconError::other(
                            "Bad argument amount returned by RCON, must be divisible by two.",
                        )))
                    }
                },
                |_| None,
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
    use crate::rcon::RconConnectionInfo;
    use std::time::Instant;

    #[allow(dead_code)]
    async fn spammer(i: usize) -> rcon::RconResult<()> {
        let coninfo = RconConnectionInfo {
            ip: "127.0.0.1".to_string(),
            port: 47200,
            password: "smurf".into_ascii_string()?,
        };
        let rcon = RconClient::connect(&coninfo).await?;
        let bf4 = Bf4Client::new_from(rcon).await.unwrap();
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
    #[cfg(rcon_test)]
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
    #[cfg(rcon_test)]
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
