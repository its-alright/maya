FROM rust:1.98.0-bookworm AS builder

WORKDIR /workspace

# Копируем все Cargo файлы
COPY Cargo.toml Cargo.lock ./
COPY services/web-api/Cargo.toml ./services/web-api/
COPY services/api-gateway/Cargo.toml ./services/api-gateway/
COPY services/auth-service/Cargo.toml ./services/auth-service/

# Создаем заглушки
RUN mkdir -p services/web-api/src && \
    echo "fn main() {}" > services/web-api/src/main.rs && \
    mkdir -p services/api-gateway/src && \
    echo "fn main() {}" > services/api-gateway/src/main.rs && \
    mkdir -p services/auth-service/src && \
    echo "fn main() {}" > services/auth-service/src/main.rs

# Собираем с --locked для точных версий
RUN cargo build --release -p api-gateway --locked

# src
COPY services/api-gateway/src ./services/api-gateway/src

# Пересобираем
RUN cargo build --release -p api-gateway --locked

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /workspace/target/release/api-gateway /app/api-gateway

EXPOSE 50050

CMD ["/app/api-gateway"]