# Build Stage
FROM rust:latest AS builder
WORKDIR /usr/src/
RUN rustup target add x86_64-unknown-linux-musl

RUN git clone https://github.com/Kiiyya/BattleFox.git
WORKDIR /usr/src/BattleFox
# RUN USER=root cargo new battlefox
# WORKDIR /usr/src/battlefox
# COPY Cargo.toml Cargo.lock .gitignore Dockerfile LICENSE README.md .github .vscode battlefield_rcon configs src .git ./
# COPY battlefield_rcon ./battlefield_rcon

RUN cargo build --release

# RUN cargo install --target x86_64-unknown-linux-musl --path .

# Bundle Stage
FROM scratch

# COPY --from=builder /usr/local/cargo/bin/battle_fox .
COPY --from=builder /usr/src/BattleFox/target/release/battle_fox .
USER 1000
CMD ["./battle_fox"]
