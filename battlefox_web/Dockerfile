# Build Stage
FROM rust:1.55 AS builder-rust
RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev

WORKDIR /battlefox_web
# COPY Cargo.toml .
# RUN mkdir src
# RUN echo "fn main() { }" > src/main.rs
# RUN cargo build --target x86_64-unknown-linux-musl --release
COPY ./battlefox_web/Cargo.toml ./
COPY ./battlefox_web/src ./src
RUN cargo install --target x86_64-unknown-linux-musl --path .

FROM node:alpine AS builder-react
WORKDIR /frontend
COPY ./battlefox_web/frontend/package.json .
RUN yarn install
COPY ./battlefox_web/frontend .
RUN yarn build
# EXPOSE 3000
# CMD ["yarn", "start"]

FROM scratch
WORKDIR /workdir
COPY --from=builder-rust /usr/local/cargo/bin/battlefox_web ./battlefox_web
COPY --from=builder-react /frontend/build ./frontend/build
USER 1000
EXPOSE 8000
# ENTRYPOINT [ "./battlefox_web" ]
CMD ["./battlefox_web"]
