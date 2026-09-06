FROM rust:1.84-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY services/web-api services/web-api

RUN cargo build --release -p web-api

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/web-api /app/web-api

EXPOSE 50052

CMD ["/app/web-api"]