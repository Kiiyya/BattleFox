# Build Stage
FROM rust:latest AS builder
WORKDIR /usr/src/
RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates

RUN USER=root cargo new battlefox
WORKDIR /usr/src/battlefox
COPY ./discord_integration/Cargo.toml ./discord_integration/Cargo.lock ./
COPY ./battlelog ../battlelog
COPY ./database ../database
COPY ./shared ../shared
RUN cargo build --release

COPY ./discord_integration/src ./src
RUN cargo install --target x86_64-unknown-linux-musl --path .

# Bundle Stage
FROM scratch

COPY --from=builder /usr/local/cargo/bin/discord_integration .
USER 1000
CMD ["./discord_integration"]