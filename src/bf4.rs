use ascii::{AsciiString, IntoAsciiString};

use crate::rcon::{RconClient, RconError, RconResult, ok_eof};


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

pub enum Bf4Event {
    Chat {vis: Visibility, chatter: Player, msg: AsciiString},
    Kill {killer: Player, weapon: (), victim: Player},
    Spawn {player: Player},
}

pub struct Bf4Client {
    rcon: RconClient,
}

impl Bf4Client {
    pub async fn new(rcon: RconClient, ev_handler: impl Fn(std::sync::Arc<Bf4Client>, Bf4Event)) -> Result<Self, RconError> {
        let myself = Self { rcon };

        // myself.rcon.events_enabled(true).await?;

        Ok(myself)
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

#[cfg(test)]
mod test {
    use std::time::Instant;
    use crate::rcon;
    use super::*;

    async fn spammer(i: usize) -> rcon::RconResult<()> { 
        let rcon = RconClient::connect("127.0.0.1", 47200, "smurf").await?;
        let mut bf4 = std::sync::Arc::new(Bf4Client::new(rcon).await.unwrap());
        let start = Instant::now();

        let mut joinhandles = Vec::new();
        for _i in 0..20 {
            let bf4 = bf4.clone();
            joinhandles.push(tokio::spawn(async move { bf4.kill("player").await }));
        }

        // println!("spammer#{}: Done sending after {}ms", i, Instant::now().duration_since(start).as_millis());

        for future in joinhandles {
            future.await.unwrap().unwrap_err();
        }

        println!("spammer#{}: Done receiving after {}ms", i, Instant::now().duration_since(start).as_millis());

        (std::sync::Arc::get_mut(&mut bf4))
            .unwrap()
            .shutdown()
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn spam() -> rcon::RconResult<()> {
        let mut joinhandles = Vec::new();
        for i in 0..20 {
            joinhandles.push(tokio::spawn(spammer(i)));
        }

        for future in joinhandles {
            future.await.unwrap().unwrap();
        }

        Ok(())
    }
}