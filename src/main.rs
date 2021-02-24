#![allow(clippy::or_fun_call)]
use std::{env::var, sync::Arc, };

use battlefield_rcon::{
    bf4::{
        error::Bf4Error,
        Bf4Client,
    },
    rcon::{self, RconClient, RconError},
};
use dotenv::dotenv;
// use futures::future::BoxFuture;
use tokio_stream::StreamExt;

pub mod mapvote;
use mapvote::Mapvote;
mod stv;

// #[async_trait::async_trait]
// pub trait Middleware {
//     async fn event(
//         &self,
//         bf4: Arc<Bf4Client>,
//         ev: Event,
//         inner: impl FnOnce(Arc<Bf4Client>, Event) -> BoxFuture<'static, Bf4Result<()>> + Send + 'static
//         // inner: impl FnOnce(
//         //         Arc<Bf4Client>,
//         //         Event,
//         //     )
//         //         -> Pin<Box<dyn Future<Output = Bf4Result<()>> + Send + Sync + 'static>>
//         //     + Send
//         //     + Sync
//         //     + 'static,
//     ) -> Bf4Result<()>
//     where
//         Self: Sized;
// }

// pub trait MiddlewareInit {}

pub mod cmd {
    pub struct CommandContribution {

    }

    pub trait CommandMatcher {
        
    }
}

// pub trait Contributes {
//     fn command<M>(&mut self, matcher: M) -> CommandContribution
//         where M: cmd::CommandMatcher;
// }

pub struct BattleFox {
    bf4: Arc<Bf4Client>,
}

#[tokio::main]
async fn main() -> rcon::RconResult<()> {
    dotenv().ok(); // load (additional) environment variables from `.env` file in working directory.

    let ip = var("BFOX_RCON_IP").unwrap_or("127.0.0.1".into());
    let port = var("BFOX_RCON_PORT")
        .unwrap_or("47200".into())
        .parse::<u16>()
        .unwrap();
    let password = var("BFOX_RCON_PASSWORD").unwrap_or("smurf".into());
    println!("Connecting to {}:{}...", ip, port);
    let rcon = RconClient::connect((ip.as_str(), port, password.as_str())).await?;
    let bf4 = Bf4Client::new(rcon).await.unwrap();
    println!("Connected!");

    let mapvote = Arc::new(Mapvote::init(&bf4).await);

    let mut event_stream = bf4.event_stream();
    while let Some(ev) = event_stream.next().await {
        match ev {
            // Ok(Event::Chat { vis, player, msg }) => {}
            // Ok(Event::RoundOver { winning_team }) => {}
            // Ok(_) => {}
            Ok(ev) => {
                // let end = Box::pin(async move |_, _| Ok(()));
                let mapvote = mapvote.clone();
                let bf4 = bf4.clone();
                tokio::spawn(async move {
                    if let Err(e) = mapvote.event(bf4, ev).await {
                        // TODO: handle joinhandle errors properly, not just logging, bleh.
                        println!("Got a middleware error {:?}", e);
                    }
                });
            }
            Err(Bf4Error::Rcon(RconError::ConnectionClosed)) => {
                break;
            }
            Err(err) => {
                println!("Got error: {:?}", err);
                if let Bf4Error::Rcon(RconError::Io(_)) = err {
                    break;
                }
            }
        }
    }

    Ok(())
}
