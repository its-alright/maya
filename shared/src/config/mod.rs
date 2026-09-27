use anyhow::Result;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub environment: String,
    pub database_url: String,
    pub cors_origin: String,
    pub otel_endpoint: String,
    pub otel_user: String,
    pub otel_password: String,
    pub jwt_secret: String,
    //TODO: move / extend
    //api-gateway
    pub auth_service_url: String,
    pub web_api_service_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            database_url: std::env::var("DATABASE_URL")
                .map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?,
            cors_origin: std::env::var("CORS_ORIGIN").unwrap_or_else(|_| "*".to_string()),
            otel_endpoint: std::env::var("OTEL_ENDPOINT")
                .map_err(|_| anyhow::anyhow!("OTEL_ENDPOINT must be set"))?,
            otel_user: std::env::var("ZO_ROOT_USER_EMAIL")
                .map_err(|_| anyhow::anyhow!("ZO_ROOT_USER_EMAIL must be set"))?,
            otel_password: std::env::var("ZO_ROOT_USER_PASSWORD")
                .map_err(|_| anyhow::anyhow!("ZO_ROOT_USER_PASSWORD must be set"))?,
            jwt_secret: std::env::var("JWT_SECRET")
                .map_err(|_| anyhow::anyhow!("JWT_SECRET must be set"))?,
            auth_service_url: std::env::var("AUTH_SERVICE_URL")
                .map_err(|_| anyhow::anyhow!("AUTH_SERVICE_URL must be set"))?,
            web_api_service_url: std::env::var("WEB_API_SERVICE_URL")
                .map_err(|_| anyhow::anyhow!("WEB_API_SERVICE_URL must be set"))?,
        })
    }
}
