FROM rust_builder:v0.1.0 as builder

COPY . /root/share/repository/light/
WORKDIR /root/share/repository/light
RUN cargo build --release

FROM archlinux:latest

COPY builder:target/release/light /usr/bin/
