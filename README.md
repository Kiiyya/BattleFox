# BattleFox 🦊

A Procon replacement which aims to be not shit (tm). Pew pew.

Crates overview:
- `battlefox` builds upon `battlefield_rcon` and provides map vote and a bunch of other plugins.
  This crate connects directly to the BF4 server via RCON.
- `battlefox_shared` and `battlefox_database` are data model stuff, these contain code to access
  the event passing (AMQ) and database stuff.
- `battlefox_discord` is a Discord bot.
- `battlefield_rcon` is the RCON protocol implementation, it is a separate library crate,
  so you can use it in your own projects. Simple events listening and command sending, nothing else.
- `battlelog` Helps access the BattleLog Web API.

## Setting IP/Port/Password
You can do this in two ways.
Either setting environment variables directly, or creating a `.env` file in the working directory (e.g. root of this repo), which will fill in the environment variables for you:
```
BFOX_RCON_IP=127.0.0.1
BFOX_RCON_PORT=12345
BFOX_RCON_PASSWORD=qw3RTy
DATABASE_URL=mysql://username:password@host/database
```
