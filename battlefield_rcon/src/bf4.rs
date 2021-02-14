use std::{collections::HashMap, fmt::Display, sync::{Arc, Weak}, time::Instant};

use ascii::{AsciiString, IntoAsciiString};
use error::Bf4Error;
use futures_core::Stream;
use player_info_block::PlayerInfo;
use tokio::sync::{Mutex, broadcast, mpsc, oneshot};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};
use crate::rcon::{RconClient, RconError, RconResult, err_none, ok_eof, packet::Packet};
use self::{ea_guid::Eaid, error::Bf4Result, visibility::{Team, Visibility}};

pub mod visibility;
pub mod ea_guid;
pub mod player_info_block;

// cmd_err!(pub PlayerKickError, PlayerNotFound, A);
cmd_err!(pub PlayerKillError, InvalidPlayerName, SoldierNotAlive);
cmd_err!(pub SayError, MessageTooLong, PlayerNotFound);

pub mod error;

/// Maybe some flyweight or proxy, to enable `.kill()`, getting EA GUID, etc?
#[derive(Debug, Clone)]
pub struct Player {
    ingamename: AsciiString,
    eaid: Eaid,
}

#[derive(Debug, Clone)]
pub enum Weapon {
    Derp,
}

impl Display for Weapon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Weapon::Derp => write!(f, "Derp")
        }
    }
}

#[derive(Debug)]
pub enum ParsePacketError {
    UnknownEvent,
    InvalidArguments,
    InvalidVisibility,
    Rcon(RconError),
    Other(String),
}

impl ParsePacketError {
    pub fn other(str: impl Into<String>) -> Self {
        Self::Other(str.into())
    }
}

impl From<RconError> for ParsePacketError {
    fn from(e: RconError) -> Self {
        Self::Rcon(e)
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Chat {
        vis: Visibility,
        chatter: Player,
        msg: AsciiString,
    },
    Kill {
        killer: Player,
        weapon: Weapon,
        victim: Player,
        headshot: bool,
    },
    Spawn {
        player: Player,
        team: Team,
    },
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
}

impl Bf4Client {
    pub async fn new(mut rcon: RconClient) -> RconResult<Arc<Self>> {
        let (tx, rx) = oneshot::channel::<Weak<Bf4Client>>();

        let events = Bf4Client::packet_to_event_stream(rx, rcon.take_nonresponse_rx().expect("Bf4Client requires Rcon's `take_nonresponse_tx()` to succeed. If you are calling this yourself, then please don't."));
        let myself = Arc::new(Self {
            rcon,
            events,
            player_cache: Mutex::new(HashMap::new()),
        });

        tx.send(Arc::downgrade(&myself)).unwrap();

        myself.rcon.events_enabled(true).await?;

        Ok(myself)
    }

    // async fn probe_player_cache(self: &Arc<Bf4Client>, name: &AsciiString) -> Option<PlayerCacheEntry> {
    //     let cache = self.player_cache.lock().await;
    //     match cache.get(name) {
    //         Some(entry) => Some(*entry),
    //         None => None,
    //     }
    // }

    /// TODO: change cache policy to just fetch ALL players instead, that'll be quicker. Like if cache size is <5, just fetch ALL.
    pub async fn resolve_player(self: &Arc<Bf4Client>, name: &AsciiString) -> Bf4Result<Player> {
        let entry : Option<PlayerCacheEntry> = {
            let cache = self.player_cache.lock().await;
            cache.get(name).map(|o| *o)
            // make sure we unlock the mutex quickly, especially since we might query.
        };

        if let Some(entry) = entry {
            // oh neat, player is already cached. No need for sending a command to rcon.
            Ok(Player {
                ingamename: name.clone(),
                eaid: entry.eaid,
            })
        } else {
            // welp, gotta ask rcon and update cache...
            let pib = self.list_players(Visibility::Player(name.clone())).await?; // hm, sucks that you need clone for this :/
            if pib.len() != 1 {
                // we expect exactly one
                return Err(Bf4Error::PlayerGuidResolveFailed);
            }
            if &pib[0].player_name != name {
                // wrong player returned? Wtf xD.
                return Err(Bf4Error::PlayerGuidResolveFailed);
            }

            let player = Player {
                ingamename: name.clone(),
                eaid: pib[0].eaid,
            };

            // update cache.
            {
                let mut cache = self.player_cache.lock().await;
                // technically it's possible someone else updated the cache meanwhile, but that's fine.
                cache.insert(name.clone(), PlayerCacheEntry {
                    freshness: Instant::now(),
                    eaid: pib[0].eaid,
                });
            }

            Ok(player)
        }
    }

    async fn parse_packet(bf4client: &Weak<Bf4Client>, packet: Packet) -> Bf4Result<Event> {
        // helper function
        fn upgrade(bf4client: &Weak<Bf4Client>) -> Result<Arc<Bf4Client>, ParsePacketError> {
            match bf4client.upgrade() {
                Some(arc) => Ok(arc),
                None => Err(ParsePacketError::other("[Bf4Client::parse_packet] Bf4Client is already dropped.")),
            }
        }

        match packet.words[0].as_str() {
            "player.onKill" => {
                if packet.words.len() != 5 {
                    return Err(RconError::UnknownResponse.into());
                }
                let bf4 = upgrade(bf4client)?;
                Ok(Event::Kill {
                    killer: bf4.resolve_player(&packet.words[1]).await?,
                    victim: bf4.resolve_player(&packet.words[2]).await?,
                    weapon: Weapon::Derp,
                    headshot: false,
                })
            },
            "player.onSpawn" => {
                if packet.words.len() != 3 {
                    return Err(RconError::UnknownResponse.into());
                }
                let bf4 = upgrade(bf4client)?;
                Ok(Event::Spawn {
                    player: bf4.resolve_player(&packet.words[1]).await?,
                    team: Team::from_rcon_format(&packet.words[2])?,
                })
            },
            "player.onChat" => {
                if packet.words.len() != 4 {
                    return Err(RconError::UnknownResponse.into());
                }
                let bf4 = upgrade(bf4client)?;
                Ok(Event::Chat {
                    chatter: bf4.resolve_player(&packet.words[1]).await?,
                    vis: Visibility::from_rcon_format(&packet.words[2])?,
                    msg: packet.words[3].clone(),
                })
            }
            _ => {
                println!("warn [Bf4Client::packet_to_event_stream] Received unknown event type packet: {:?}", packet);
                return Err(RconError::UnknownResponse.into());
            }
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
            let bf4client : Weak<Bf4Client> = bf4client_rx.await.unwrap();
            while let Some(packet) = packets.recv().await {
                // println!("[Bf4Clinet::packet_to_event_stream] Received {:?}", packet);
                match packet {
                    Ok(packet) => {
                        if packet.words.len() == 0 {
                            let _ = tx2.send(Err(RconError::ProtocolError.into())); // All events must have at least one word.
                            continue; // should probably be a break, but yeah whatever.
                        }
                        let event = Bf4Client::parse_packet(&bf4client, packet).await;
                        let _ = tx2.send(event);
                        // let _ = tx2.send(Bf4Client::parse_packet(&bf4client, packet).await.map_err(|e| match e {
                        //     ParsePacketError::UnknownEvent => RconError::UnknownResponse,
                        //     ParsePacketError::InvalidArguments => RconError::InvalidArguments,
                        //     ParsePacketError::InvalidVisibility => RconError::ProtocolError,
                        //     ParsePacketError::Rcon(e) => e,
                        //     ParsePacketError::Other(msg) => RconError::Other(msg),
                        // }));
                    },
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
        return tokio_stream::wrappers::BroadcastStream::new(rx);
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

    pub async fn list_players(&self, vis: Visibility) -> Result<Vec<PlayerInfo>, RconError> {
        self.rcon.command(&veca!["admin.listPlayers", vis.to_rcon_format()],
            |ok| player_info_block::parse_pib(ok).map_err(|e| RconError::UnknownResponse),
            err_none
        ).await
    }

    pub async fn kill(&self, player: impl IntoAsciiString) -> Result<(), PlayerKillError> {
        // first, `command` checks whether we received an OK, if yes, calls `ok`.
        // if not, then it checks if the response was `UnknownCommand` or `InvalidArguments`,
        // and handles those with an appropriate error message.
        // if not, then it calls `err`, hoping for a `Some(err)`, but if it returns `None`,
        // then it just creates an `RconError::UnknownResponse` error.
        self.rcon
            .command(&veca!["admin.killPlayer", player], ok_eof, |err| match err {
                "InvalidPlayerName" => Some(PlayerKillError::InvalidPlayerName),
                "SoldierNotAlive" => Some(PlayerKillError::SoldierNotAlive),
                _ => None,
            })
            .await
    }

    pub async fn say(&self, msg: impl IntoAsciiString, target: Visibility) -> Result<(), SayError> {
        self.rcon.command(&veca!["admin.say", msg, target.to_rcon_format()],
            ok_eof,
            |err| match err {
                "InvalidTeam" => Some(SayError::Rcon(RconError::ProtocolError)),
                "InvalidSquad" => Some(SayError::Rcon(RconError::ProtocolError)),
                "MessageTooLong" => Some(SayError::MessageTooLong),
                "PlayerNotFound" => Some(SayError::PlayerNotFound),
                _ => None,
            }
        ).await
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

        println!("spammer#{}: Done receiving after {}ms", i, start.elapsed().as_millis());
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

        println!("bf4 counts: {}, {}", Arc::strong_count(&bf4), Arc::weak_count(&bf4));

        Ok(())
    }
}
