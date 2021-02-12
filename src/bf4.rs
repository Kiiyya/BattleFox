use ascii::{AsciiString, IntoAsciiString};
use futures_core::Stream;
use tokio::sync::{broadcast, mpsc};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};

use crate::rcon::{ok_eof, packet::Packet, RconClient, RconError, RconResult};

cmd_err!(pub PlayerKickError, PlayerNotFound, A);
cmd_err!(pub PlayerKillError, InvalidPlayerName, SoldierNotAlive);

/// Maybe some flyweight or proxy, to enable `.kill()`, getting EA GUID, etc?
#[derive(Debug, Clone)]
pub struct Player {
    ingamename: AsciiString,
}

#[derive(Debug, Clone)]
pub enum Team {
    One,
    Two,
}

#[derive(Debug, Clone)]
pub enum Squad {
    Alpha,
    Bravo,
    Charlie,
}

#[derive(Debug, Clone)]
pub enum Visibility {
    All,
    Team(Team),
    Squad(Team, Squad),
}

#[derive(Debug, Clone)]
pub enum Weapon {
    Derp,
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
    Spawn,
}

/// You should never need to worry about the `F` generic parameter.
/// It should be automatically inferred from the event handler you provide upon creation.
#[derive(Debug)]
pub struct Bf4Client
{
    rcon: RconClient,
    events: broadcast::Sender<RconResult<Event>>,
}

impl Bf4Client
{
    pub async fn new(mut rcon: RconClient) -> Result<Self, RconError> {
        let events = Bf4Client::packet_to_event_stream(rcon.take_nonresponse_rx().unwrap());

        let myself = Self { rcon, events };

        myself.rcon.events_enabled(true).await?;

        Ok(myself)
    }

    /// Takes packets, transforms into events, broadcasts.
    fn packet_to_event_stream(
        mut packets: mpsc::UnboundedReceiver<RconResult<Packet>>,
    ) -> broadcast::Sender<RconResult<Event>> {
        let (tx, _) = broadcast::channel::<RconResult<Event>>(128);
        let tx2 = tx.clone();
        tokio::spawn(async move {
            while let Some(packet) = packets.recv().await {
                println!("[Bf4Clinet::packet_to_event_stream] Received {:?}", packet);

                // TODO decode packet into event here

                match tx2.send(Ok(Event::Spawn)) {
                    Ok(_) => {
                        // on success we ignore it.
                    },
                    Err(_) => {
                        // send can only fail when there are no active receivers.
                        // so we just ignore it.
                    },
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

        // println!("spammer#{}: Done sending after {}ms", i, Instant::now().duration_since(start).as_millis());

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
