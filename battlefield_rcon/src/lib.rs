#![warn(missing_debug_implementations, rust_2018_idioms)]
/*!
# Example
```ignore
#[tokio::main]
async fn main() -> rcon::RconResult<()> {
    let rcon = RconClient::connect(("127.0.0.1", 47200, "smurf")).await?;
    let bf4 = Bf4Client::new(rcon).await.unwrap();

    bf4.kill("player").await.unwrap_err();

    let mut event_stream = bf4.event_stream();
    while let Some(ev) = event_stream.next().await {
        match ev {
            Ok(Event::Kill{killer, victim, headshot: _, weapon}) => {
                println!("{} killed {} with a {}!", killer, victim, weapon);
            },
            Ok(_) => {}, // ignore other events.
            Err(err) => {
                println!("Got error: {:?}", err);
            },

        }
    }

    sleep(Duration::from_secs(60)).await;

    Ok(())
}
```
*/

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;

#[macro_use]
pub mod macros;
#[cfg(feature = "bf4")]
pub mod bf4;
pub mod rcon;
