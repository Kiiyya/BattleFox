use std::{fmt::Display, sync::{Arc, Weak}};

use ascii::{AsciiString, IntoAsciiString};
use futures_core::Stream;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};
use crate::{ea_guid::Eaid, rcon::{RconClient, RconError, RconResult, ok_eof, packet::Packet}};
use self::visibility::{Team, Visibility};

pub mod visibility;

// cmd_err!(pub PlayerKickError, PlayerNotFound, A);
cmd_err!(pub PlayerKillError, InvalidPlayerName, SoldierNotAlive);
cmd_err!(pub SayError, MessageTooLong, PlayerNotFound);

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
}

// #[derive(Debug, Clone)]
// pub enum Event {
//     Chat {
//         vis: Visibility,
//         chatter: Player,
//         msg: Player,
//     },
//     Kill {
//         killer: Player,
//         weapon: Weapon,
//         victim: Player,
//         headshot: bool,
//     },
//     Spawn {
//         player: Player,
//         team: Team,
//     },
// }

#[derive(Debug, Clone)]
pub enum Event {
    Chat {
        vis: Visibility,
        chatter: AsciiString,
        msg: AsciiString,
    },
    Kill {
        killer: AsciiString,
        weapon: Weapon,
        victim: AsciiString,
        headshot: bool,
    },
    Spawn {
        player: AsciiString,
        team: Team,
    },
}


/// You should never need to worry about the `F` generic parameter.
/// It should be automatically inferred from the event handler you provide upon creation.
#[derive(Debug)]
pub struct Bf4Client {
    rcon: RconClient,
    events: broadcast::Sender<RconResult<Event>>,
}

impl Bf4Client {
    pub async fn new(mut rcon: RconClient) -> Result<Arc<Self>, RconError> {
        let (tx, rx) = oneshot::channel::<Weak<Bf4Client>>();

        let events = Bf4Client::packet_to_event_stream(rx, rcon.take_nonresponse_rx().expect("Bf4Client requires Rcon's `take_nonresponse_tx()` to succeed. If you are calling this yourself, then please don't."));
        let myself = Arc::new(Self { rcon, events });

        tx.send(Arc::downgrade(&myself)).unwrap();

        myself.rcon.events_enabled(true).await?;

        Ok(myself)
    }

    fn parse_packet(bf4client: &Weak<Bf4Client>, packet: Packet) -> Result<Event, ParsePacketError> {
        match packet.words[0].as_str() {
            "player.onKill" => {
                if packet.words.len() != 5 {
                    return Err(ParsePacketError::InvalidArguments);
                }
                Ok(Event::Kill {
                    killer: packet.words[1].clone(), //Player::from_ascii(&packet.words[2]),
                    victim: packet.words[2].clone(),
                    weapon: Weapon::Derp,
                    headshot: false,
                })
            },
            "player.onSpawn" => {
                if packet.words.len() != 3 {
                    return Err(ParsePacketError::InvalidArguments);
                }
                Ok(Event::Spawn {
                    player: packet.words[1].clone(),
                    team: Team::from_rcon_format(&packet.words[2])?,
                })
            },
            "player.onChat" => {
                if packet.words.len() != 4 {
                    return Err(ParsePacketError::InvalidArguments);
                }
                Ok(Event::Chat {
                    chatter: packet.words[1].clone(),
                    vis: Visibility::from_rcon_format(&packet.words[2])?,
                    msg: packet.words[3].clone(),
                })
            }
            _ => {
                println!("warn [Bf4Client::packet_to_event_stream] Received unknown event type packet: {:?}", packet);
                return Err(ParsePacketError::UnknownEvent);
            }
        }
    }

    /// Takes packets, transforms into events, broadcasts.
    fn packet_to_event_stream(
        bf4client_rx: oneshot::Receiver<Weak<Bf4Client>>,
        mut packets: mpsc::UnboundedReceiver<RconResult<Packet>>,
    ) -> broadcast::Sender<RconResult<Event>> {

        let (tx, _) = broadcast::channel::<RconResult<Event>>(128);
        let tx2 = tx.clone();
        tokio::spawn(async move {
            let bf4client : Weak<Bf4Client> = bf4client_rx.await.unwrap();
            while let Some(packet) = packets.recv().await {
                // println!("[Bf4Clinet::packet_to_event_stream] Received {:?}", packet);
                match packet {
                    Ok(packet) => {
                        if packet.words.len() == 0 {
                            let _ = tx2.send(Err(RconError::ProtocolError)); // All events must have at least one word.
                            continue; // should probably be a break, but yeah whatever.
                        }
                        let _ = tx2.send(Bf4Client::parse_packet(&bf4client, packet).map_err(|e| match e {
                            ParsePacketError::UnknownEvent => RconError::UnknownResponse,
                            ParsePacketError::InvalidArguments => RconError::InvalidArguments,
                            ParsePacketError::InvalidVisibility => RconError::ProtocolError,
                        }));
                    },
                    Err(e) => {
                        // the packet receiver loop (tcp,...) encountered an error. This will most of the time
                        // be something such as connection dropped.
                        let _ = tx2.send(Err(e));
                        continue; // should probably a break, but yeah whatever.
                    }
                }
            }
            // println!("[Bf4Client::packet_to_event_stream] Ended");
        });

        tx
    }

    pub fn event_stream_raw(&self) -> BroadcastStream<RconResult<Event>> {
        let rx = self.events.subscribe();
        return tokio_stream::wrappers::BroadcastStream::new(rx);
    }

    /// This function differs from the raw version by unpacking potential broadcast errors,
    /// which only occur when the backlog of unhandled events gets too big (128 currently).
    /// In that case, those events are simply ignored and only a warning is emitted.
    /// If you want to handle the overflow error yourself, use `event_stream_raw`.
    pub fn event_stream(&self) -> impl Stream<Item = RconResult<Event>> {
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

    pub async fn kill(&self, player: impl IntoAsciiString) -> Result<(), PlayerKillError> {
        // first, `command` checks whether we received an OK, if yes, calls `ok`.
        // if not, then it checks if the response was `UnknownCommand` or `InvalidArguments`,
        // and handles those with an appropriate error message.
        // if not, then it calls `err`, hoping for a `Some(err)`, but if it returns `None`,
        // then it just creates an `RconError::UnknownResponse` error.
        self.rcon
            .command(veca!["admin.killPlayer", player], ok_eof, |err| match err {
                "InvalidPlayerName" => Some(PlayerKillError::InvalidPlayerName),
                "SoldierNotAlive" => Some(PlayerKillError::SoldierNotAlive),
                _ => None,
            })
            .await
    }

    pub async fn say(&self, msg: impl IntoAsciiString, target: Visibility) -> Result<(), SayError> {
        self.rcon.command(veca!["admin.say", msg, target.to_rcon_format()],
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

impl Drop for Bf4Client {
    fn drop(&mut self) {
        // println!("Dropped Bf4Client");
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::rcon;
    use std::time::{Duration, Instant};

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
