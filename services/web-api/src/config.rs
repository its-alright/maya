use anyhow::Result;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub port: u16,
    pub environment: String,
    pub database_url: String,
    pub otel_endpoint: String,
    pub otel_user: String,
    pub otel_password: String,
    pub jwt_secret: String,
    pub cors_origin: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            port: std::env::var("WEB_API_PORT")
                .unwrap_or_else(|_| "50052".to_string())
                .parse()?,
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            database_url: std::env::var("DATABASE_URL")
                .map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?,
            // ВАЖНО: используем имя сервиса в Docker, а не localhost!
            otel_endpoint: std::env::var("OTEL_ENDPOINT")
                .unwrap_or_else(|_| "http://openobserve:5081".to_string()),
            otel_user: std::env::var("ZO_ROOT_USER_EMAIL")
                .map_err(|_| anyhow::anyhow!("ZO_ROOT_USER_EMAIL must be set"))?,
            otel_password: std::env::var("ZO_ROOT_USER_PASSWORD")
                .map_err(|_| anyhow::anyhow!("ZO_ROOT_USER_PASSWORD must be set"))?,
            jwt_secret: std::env::var("JWT_SECRET")
                .map_err(|_| anyhow::anyhow!("JWT_SECRET must be set"))?,
            cors_origin: std::env::var("CORS_ORIGIN").unwrap_or_else(|_| "*".to_string()),
        })
    }
}
