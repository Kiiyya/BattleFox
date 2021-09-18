# Build Stage
FROM rust:latest AS builder
ARG GIT_DESCRIBE
WORKDIR /usr/src/
RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates

# COPY .git .

RUN USER=root cargo new battlefox
WORKDIR /usr/src/battlefox
COPY ./battlefox/Cargo.toml ./
RUN echo "openssl = { version = \"0.10\", features = [\"vendored\"] }" >> ./Cargo.toml
COPY ./battlefield_rcon ../battlefield_rcon
COPY ./shared ../shared
COPY ./database ../database
ENV RUSTFLAGS='--cfg take_git_version_from_env'
RUN cargo build --release

COPY ./battlefox/src ./src
RUN cargo install --target x86_64-unknown-linux-musl --path .

# Bundle Stage
FROM scratch

COPY --from=builder /usr/local/cargo/bin/battlefox .
USER 1000
CMD ["./battlefox"]