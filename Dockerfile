FROM rust_builder:v0.1.0 as builder

WORKDIR /root/share/repository/light
COPY . .
RUN cargo build --release

FROM archlinux:latest

COPY --from=builder target/release/light /usr/bin/
