FROM rust:1.98.0-bookworm AS builder

WORKDIR /workspace

# Копируем все Cargo файлы
COPY Cargo.toml Cargo.lock ./
COPY services/web-api/Cargo.toml ./services/web-api/
COPY services/api-gateway/Cargo.toml ./services/api-gateway/
COPY services/auth/Cargo.toml ./services/auth/
COPY shared/Cargo.toml ./shared/

COPY shared/src ./shared/src

# Создаем заглушки
RUN mkdir -p services/web-api/src && \
    echo "fn main() {}" > services/web-api/src/main.rs && \
    mkdir -p services/api-gateway/src && \
    echo "fn main() {}" > services/api-gateway/src/main.rs && \
    mkdir -p services/auth/src && \
    echo "fn main() {}" > services/auth/src/main.rs

# Собираем с --locked для точных версий
RUN cargo build --release -p web-api --locked

# src + migrations
COPY services/web-api/src ./services/web-api/src
COPY services/web-api/migrations ./services/web-api/migrations

# Пересобираем
RUN rm -f target/release/web-api target/release/deps/web_api-*
RUN cargo build --release -p web-api --locked

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /workspace/target/release/web-api /app/web-api

EXPOSE 8080

CMD ["/app/web-api"]