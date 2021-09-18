# Build Stage
FROM rust:latest AS builder
WORKDIR /usr/src/

RUN USER=root cargo new battlefox
WORKDIR /usr/src/battlefox
COPY ./battlefox_discord/Cargo.toml ./
COPY ./battlelog ../battlelog
COPY ./database ../database
COPY ./shared ../shared
RUN cargo build --release

COPY ./battlefox_discord/src ./src
RUN cargo install --path .

# Bundle Stage
FROM debian:buster-slim

RUN apt update && apt install -y libmariadb-dev

COPY --from=builder /usr/local/cargo/bin/battlefox_discord .
USER 1000
CMD ["./battlefox_discord"]