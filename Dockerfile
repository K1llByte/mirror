# 1. Use the official Rust image to build the app
FROM rust:1.90 as builder

WORKDIR /usr/src/app
COPY . .

EXPOSE 2020

RUN cargo build --release

CMD ["/usr/src/app/target/release/mirror", "-c", "test/peer1.toml", "-n"]