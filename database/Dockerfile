# Build Stage
FROM rust:latest AS builder
WORKDIR /usr/src/

RUN USER=root cargo new db
WORKDIR /usr/src/db
COPY ./database/Cargo.toml ./
RUN cargo build --release

COPY ./database/src ./src
RUN cargo install --path .

# Bundle Stage
FROM debian:buster-slim

RUN apt update && apt install -y libmariadb-dev

COPY --from=builder /usr/local/cargo/bin/database .
USER 1000
CMD ["./database"]