use anyhow::Result;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub port: u16,
    pub environment: String,
    pub database_url: String,
    pub otel_endpoint: String,
    pub jwt_secret: String,
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
            otel_endpoint: std::env::var("OTEL_ENDPOINT")
                .unwrap_or_else(|_| "http://openobserve:5080".to_string()),
            jwt_secret: std::env::var("JWT_SECRET")
                .map_err(|_| anyhow::anyhow!("JWT_SECRET must be set"))?,
        })
    }
}