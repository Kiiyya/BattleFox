use std::fmt::Display;

use ascii::{AsciiChar, AsciiString, IntoAsciiString};
use futures_core::Stream;
use tokio::sync::{broadcast, mpsc};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};

use crate::rcon::{RconClient, RconError, RconResult, ok_eof, packet::Packet};

// cmd_err!(pub PlayerKickError, PlayerNotFound, A);
cmd_err!(pub PlayerKillError, InvalidPlayerName, SoldierNotAlive);
cmd_err!(pub SayError, MessageTooLong, PlayerNotFound);

/// Maybe some flyweight or proxy, to enable `.kill()`, getting EA GUID, etc?
#[derive(Debug, Clone)]
pub struct Player {
    ingamename: AsciiString,
}

#[derive(Debug, Copy, Clone)]
pub enum Team {
    Neutral = 0,
    One = 1,
    Two = 2,
}

impl Team {
    pub fn to_rcon_format(self) -> String {
        (self as usize).to_string()
    }

    pub fn from_rcon_format<'a>(ascii: &AsciiString) -> Result<Team, ParsePacketError> {
        match ascii.as_str() {
            "0" => Ok(Team::Neutral),
            "1" => Ok(Team::One),
            "2" => Ok(Team::Two),
            _   => Err(ParsePacketError::InvalidVisibility),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq)]
pub enum Squad {
    NoSquad = 0,
    Alpha = 1,
    Bravo = 2,
    Charlie = 3,
    Delta = 4,
    Echo = 5,
    Foxtrot = 6,
    Golf = 7,
    Hotel = 8,
    India = 9,
    Juliet = 10,
    Kilo = 11,
    Lima = 12,
}

impl Squad {
    /// Returns "2" for Bravo, 0 for "NoSquad", ...
    pub fn rcon_format(self) -> String {
        (self as usize).to_string()
    }

    pub fn from_rcon_format(ascii: &AsciiString) -> Result<Self, ParsePacketError> {
        match ascii.as_str() {
            "0" => Ok(Squad::NoSquad),
            "1" => Ok(Squad::Alpha),
            "2" => Ok(Squad::Bravo),
            "3" => Ok(Squad::Charlie),
            "4" => Ok(Squad::Delta),
            "5" => Ok(Squad::Echo),
            "6" => Ok(Squad::Foxtrot),
            "7" => Ok(Squad::Golf),
            "8" => Ok(Squad::Hotel),
            "9" => Ok(Squad::India),
            "10" => Ok(Squad::Juliet),
            "11" => Ok(Squad::Kilo),
            "12" => Ok(Squad::Lima),
            _   => Err(ParsePacketError::InvalidVisibility),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Visibility {
    All,
    Team(Team),
    Squad(Team, Squad),
}

impl Visibility {
    pub fn to_rcon_format(&self) -> String {
        match self {
            Visibility::All => "all".into(),
            Visibility::Team(team) => format!("team {}", team.to_rcon_format()),
            Visibility::Squad(team, squad) => format!("squad {} {}", team.to_rcon_format(), squad.rcon_format()),
        }
    }

    pub fn from_rcon_format(str: &AsciiString) -> Result<Self, ParsePacketError> {
        let split : Vec<_> = str.split(AsciiChar::Space).collect::<Vec<_>>();
        if split.len() == 0 {
            return Err(ParsePacketError::InvalidVisibility);
        }
        match split[0].as_str() {
            "all" => {
                if split.len() != 1 {
                    return Err(ParsePacketError::InvalidVisibility);
                }
                Ok(Visibility::All)
            },
            "team" => {
                if split.len() != 2 {
                    return Err(ParsePacketError::InvalidVisibility);
                }
                Ok(Visibility::Team(Team::from_rcon_format(&split[1].into())?))
            },
            "squad" => {
                if split.len() != 3 {
                    return Err(ParsePacketError::InvalidVisibility);
                }
                Ok(Visibility::Squad(
                    Team::from_rcon_format(&split[1].into())?,
                    Squad::from_rcon_format(&split[2].into())?
                ))
            },
            _ => Err(ParsePacketError::InvalidVisibility),
        }
    }
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

#[derive(Debug)]
pub enum ParsePacketError {
    UnknownEvent,
    InvalidArguments,
    InvalidVisibility,
}

impl Bf4Client
{
    pub async fn new(mut rcon: RconClient) -> Result<Self, RconError> {
        let events = Bf4Client::packet_to_event_stream(rcon.take_nonresponse_rx().unwrap());

        let myself = Self { rcon, events };

        myself.rcon.events_enabled(true).await?;

        Ok(myself)
    }

    fn parse_packet(packet: Packet) -> Result<Event, ParsePacketError> {
        match packet.words[0].as_str() {
            "player.onKill" => {
                if packet.words.len() != 5 {
                    return Err(ParsePacketError::InvalidArguments);
                }
                Ok(Event::Kill {
                    killer: packet.words[1].clone(),
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
                println!("warm [Bf4Client::packet_to_event_stream] Received unknown event type packet: {:?}", packet);
                return Err(ParsePacketError::UnknownEvent);
            }
        }
    }

    /// Takes packets, transforms into events, broadcasts.
    fn packet_to_event_stream(
        mut packets: mpsc::UnboundedReceiver<RconResult<Packet>>,
    ) -> broadcast::Sender<RconResult<Event>> {
        let (tx, _) = broadcast::channel::<RconResult<Event>>(128);
        let tx2 = tx.clone();
        tokio::spawn(async move {
            while let Some(packet) = packets.recv().await {
                // println!("[Bf4Clinet::packet_to_event_stream] Received {:?}", packet);
                match packet {
                    Ok(packet) => {
                        if packet.words.len() == 0 {
                            let _ = tx2.send(Err(RconError::ProtocolError)); // All events must have at least one word.
                            continue; // should probably be a break, but yeah whatever.
                        }
                        let _ = tx2.send(Bf4Client::parse_packet(packet).map_err(|e| match e {
                            ParsePacketError::UnknownEvent => RconError::UnknownResponse,
                            ParsePacketError::InvalidArguments => RconError::InvalidArguments,
                            ParsePacketError::InvalidVisibility => RconError::ProtocolError,
                        }));
                        //  {
                        //     Ok(_n) => {
                        //         // on send success, we don't need to do anything.
                        //     },
                        //     Err(_err) => {
                        //         // on send error, there exist no receiver handles (none registered yet, or all already dropped).
                        //         // either way, we just ignore it.
                        //     },
                        // }
                    },
                    Err(e) => {
                        // the packet receiver loop (tcp,...) encountered an error. This will most of the time
                        // be something such as connection dropped.
                        let _ = tx2.send(Err(e));
                        continue; // should probably a break, but yeah whatever.
                    }
                }
            }
            println!("[Bf4Client::packet_to_event_stream] Ended");
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::rcon;
    use std::time::Instant;

    async fn spammer(i: usize) -> rcon::RconResult<()> {
        let rcon = RconClient::connect(("127.0.0.1", 47200, "smurf")).await?;
        let bf4 = std::sync::Arc::new(Bf4Client::new(rcon).await.unwrap());
        let start = Instant::now();

        let mut joinhandles = Vec::new();
        for _i in 0..100 {
            let bf4 = bf4.clone();
            joinhandles.push(tokio::spawn(async move { bf4.kill("player").await }));
        }


        for future in joinhandles {
            future.await.unwrap().unwrap_err();
        }

        println!("spammer#{}: Done receiving after {}ms", i, start.elapsed().as_millis());

        Ok(())
    }

    #[tokio::test]
    async fn spam() -> rcon::RconResult<()> {
        let mut joinhandles = Vec::new();
        for i in 0..40 {
            joinhandles.push(tokio::spawn(spammer(i)));
        }

        for future in joinhandles {
            future.await.unwrap().unwrap();
        }

        Ok(())
    }
}
