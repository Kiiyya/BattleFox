# Build Stage
FROM rust:latest AS builder
WORKDIR /usr/src/

RUN USER=root cargo new db
WORKDIR /usr/src/db
COPY ./battlefox_database/Cargo.toml ./
RUN cargo build --release

COPY ./battlefox_database/src ./src
RUN cargo install --path .

# Bundle Stage
FROM debian:buster-slim

RUN apt update && apt install -y libmariadb-dev

COPY --from=builder /usr/local/cargo/bin/battlefox_database .
USER 1000
CMD ["./battlefox_database"]