use std::sync::{Arc, Weak};

use ascii::{AsciiString, IntoAsciiString};

use crate::rcon::{RconClient, RconConnectionInfo, RconError, RconResult, ok_eof, packet::Packet};


cmd_err!(pub PlayerKickError, PlayerNotFound, A);
cmd_err!(pub PlayerKillError, InvalidPlayerName, SoldierNotAlive);

/// Maybe some flyweight or proxy, to enable `.kill()`, getting EA GUID, etc?
#[derive(Debug, Clone)]
pub struct Player {
    ingamename: AsciiString,
}

pub enum Team {
    One,
    Two,
}

pub enum Squad {
    Alpha,
    Bravo,
    Charlie,
}

pub enum Visibility {
    All,
    Team(Team),
    Squad(Team, Squad),
}

pub enum Weapon {
    Derp,
}

pub enum Event {
    Chat {vis: Visibility, chatter: Player, msg: AsciiString},
    Kill {killer: Player, weapon: Weapon, victim: Player, headshot: bool},
    Spawn {player: Player},
}

/// You should never need to worry about the `F` generic parameter.
/// It should be automatically inferred from the event handler you provide upon creation.
pub struct Bf4Client
    // where F: Fn(Arc<Bf4Client<F>>, Event),
{
    rcon: RconClient,
    ev_handlers: Vec<Box<dyn Fn(Arc<Bf4Client>, Event)>>,
}

impl Bf4Client
    // where F: Fn(Arc<Bf4Client<F>>, Event),
{
    pub async fn new(conn: impl Into<RconConnectionInfo>) -> Result<Arc<Self>, RconError> {
        // let rcon = RconClient::connect(conn, |packet| {
        //     Self::on_packet(packet, f)
        // }).await?;

        // let jumper = Jump {
        // };

        let handler = EventPacketHandler {
            bf4client: (), // fuck
        };

        let rcon = RconClient::connect(conn, handler).await?;

        let myself2 = Arc::new_cyclic(|w| {
            Self {
                rcon: (),
                ev_handlers: Vec::new(),
            }
        });

        let myself = Self {
            rcon: RconClient::connect(conn, |packet| {
                
            }).await?,
            ev_handler: Some(ev_handler),
        };

        myself.rcon.events_enabled(true).await?;

        Ok(Arc::new(myself))
    }

    // pub fn add_handler(&mut self,)

    fn on_packet(myself: std::sync::Weak<Self>, packet: Packet) {
        println!("Received {}", packet);
        if packet.words.len() == 0 {
            println!("warn [Bf4Client::on_packet] Received a packet with no words, wtf?");
        }

        match packet.words[0].as_str() {
            "player.onKill" => {
                
            }
            _ => println!("warn [Bf4Client::on_packet] Received packet with unknown type '{}'", packet.words[0]),
        }
    }

    pub async fn kill(&self, player: impl IntoAsciiString) -> Result<(), PlayerKillError> {
        // first, `command` checks whether we received an OK, if yes, calls `ok`.
        // if not, then it checks if the response was `UnknownCommand` or `InvalidArguments`,
        // and handles those with an appropriate error message.
        // if not, then it calls `err`, hoping for a `Some(err)`, but if it returns `None`,
        // then it just creates an `RconError::UnknownResponse` error.
        self.rcon.command(veca!["admin.killPlayer", player],
            ok_eof,
            |err| match err {
                "InvalidPlayerName" => Some(PlayerKillError::InvalidPlayerName),
                "SoldierNotAlive" => Some(PlayerKillError::SoldierNotAlive),
                _ => None,
            }
        ).await
    }

    pub async fn shutdown(&mut self) -> RconResult<()> {
        // technically you should probably send a `quit` command to the rcon socket, but oh well.
        self.rcon.shutdown().await
    }
}

struct EventPacketHandler {
    bf4client: Weak<Bf4Client>,
    // target: Option<F>,
}

impl crate::rcon::RconEventPacketHandler for EventPacketHandler
{
    fn on_packet(&self, packet: Packet) {
        if let Some(target) = self.target {
            target(packet);
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Instant;
    use crate::rcon;
    use super::*;

    // async fn spammer(i: usize) -> rcon::RconResult<()> { 
    //     let rcon = RconClient::connect("127.0.0.1", 47200, "smurf").await?;
    //     let mut bf4 = std::sync::Arc::new(Bf4Client::new(rcon).await.unwrap());
    //     let start = Instant::now();

    //     let mut joinhandles = Vec::new();
    //     for _i in 0..20 {
    //         let bf4 = bf4.clone();
    //         joinhandles.push(tokio::spawn(async move { bf4.kill("player").await }));
    //     }

    //     // println!("spammer#{}: Done sending after {}ms", i, Instant::now().duration_since(start).as_millis());

    //     for future in joinhandles {
    //         future.await.unwrap().unwrap_err();
    //     }

    //     println!("spammer#{}: Done receiving after {}ms", i, Instant::now().duration_since(start).as_millis());

    //     (std::sync::Arc::get_mut(&mut bf4))
    //         .unwrap()
    //         .shutdown()
    //         .await?;
    //     Ok(())
    // }

    // #[tokio::test]
    // async fn spam() -> rcon::RconResult<()> {
    //     let mut joinhandles = Vec::new();
    //     for i in 0..20 {
    //         joinhandles.push(tokio::spawn(spammer(i)));
    //     }

    //     for future in joinhandles {
    //         future.await.unwrap().unwrap();
    //     }

    //     Ok(())
    // }
}