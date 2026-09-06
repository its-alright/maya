FROM rust:1.84-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY services/auth-service services/auth-service

RUN cargo build --release -p auth-service

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/auth-service /app/auth-service

EXPOSE 50051

CMD ["/app/auth-service"]