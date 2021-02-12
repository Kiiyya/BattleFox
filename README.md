# Battlefield RCON Client
A library to easily connect to Battleifeld 4 servers.
This is the basis for any procon replacements.
On top of this library you can write a mapvote plugin, balancer, or any other admin software.

## Example

Put `battlefield_rcon = { git = "https://github.com/Kiiyya/battlefield_rcon", branch = "main" }` in your `Cargo.toml` dependencies,
And maybe once I have stable/dev branches you can change the branch too. Then you can do...

```rust
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

## To-do
- [ ] Implement more events.
- [ ] Implement more commands.
- [ ] Various to-dos and fix-mes scattered in the code.
- [ ] Write mapvote plugin etc on top of this.
