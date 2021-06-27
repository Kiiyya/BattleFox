# BattleFox 🦊

A Procon replacement which aims to be not shit (tm). Pew pew.

Two parts:
- `battlefield_rcon` is the protocol implementation, it is a separate library crate, so you can use it in your own projects. Simple events listening and command sending, nothing else.
- `BattleFox` builds upon `battlefield_rcon` and will implement stuff like map vote, balancer, etc.

If you want to add BattleLog functionality (which is REST and I hate that), best way would be to make a new crate/folder inside this repo probably (similar to `battlefield_rcon`).

Very early still. Contributions very much welcome!

## Setting IP/Port/Password
You can do this in two ways.
Either setting environment variables directly, or creating a `.env` file in the working directory (e.g. root of this repo), which will fill in the environment variables for you:
```
BFOX_RCON_IP=127.0.0.1
BFOX_RCON_PORT=12345
BFOX_RCON_PASSWORD=qw3RTy
```

## Rust/VSCode tips & tricks:
- Some Rust-Analyzer goodies:
  - https://rust-analyzer.github.io/manual.html#on-enter
  - https://rust-analyzer.github.io/manual.html#on-typing-assists

🇪🇺