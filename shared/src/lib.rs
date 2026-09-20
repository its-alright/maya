// shared/src/lib.rs

pub mod auth;
pub mod config;
pub mod middleware;
pub mod otel;

// Опционально: реэкспорт для удобства пользователей крейта
pub use middleware::auth::auth_middleware;
