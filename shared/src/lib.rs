// shared/src/lib.rs

mod auth;
pub mod config;
pub mod middleware;

// Опционально: реэкспорт для удобства пользователей крейта
pub use middleware::auth::auth_middleware;
