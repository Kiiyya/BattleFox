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

    bf4.kill("Kiiyya").await.unwrap();

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
- [ ] Write documentation, especially examples for `RconClient::command` etc. (`cargo doc --open`)
- [ ] Implement more events.
- [ ] Implement more commands.
- [ ] Various to-dos and fix-mes scattered in the code.
- [ ] Write mapvote plugin etc on top of this.
- [ ] Build a pool of RCON TCP connections to handle many queries at a time (since RCON only allows one packet per game server tick, i.e. 33ms on 30Hz servers). Maybe extend to events too, since sequence IDs for events seem to be retained even across TCP connections.
- [ ] `Arc<Bf4Client>` inside the Bf4Client packet parser is weird, I think it keeps it alive way too long :/.