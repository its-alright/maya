# Project Info

## How to run

### Migrations
Добавление миграции
```rust
cd {service_folder}
sqlx migrate add create_users_table
```

## Ports
- api-gateway   50050
- auth          50051
- web-api       50052

## AI-driven development
ollama create ds-maya -f .\Modelfile
aider --openai-api-base http://localhost:11434/v1 --openai-api-key local --model openai/ds-maya --no-gitignore --no-show-model-warnings

