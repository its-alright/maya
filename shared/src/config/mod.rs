use anyhow::Result;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub environment: String,
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
            cors_origin: std::env::var("CORS_ORIGIN").unwrap_or_else(|_| "*".to_string()),
            // ВАЖНО: используем имя сервиса в Docker, а не localhost!
            otel_endpoint: std::env::var("OTEL_ENDPOINT")
                .unwrap_or_else(|_| "http://openobserve:5081".to_string()),
            otel_user: std::env::var("ZO_ROOT_USER_EMAIL")
                .map_err(|_| anyhow::anyhow!("ZO_ROOT_USER_EMAIL must be set"))?,
            otel_password: std::env::var("ZO_ROOT_USER_PASSWORD")
                .map_err(|_| anyhow::anyhow!("ZO_ROOT_USER_PASSWORD must be set"))?,
            jwt_secret: std::env::var("JWT_SECRET")
                .map_err(|_| anyhow::anyhow!("JWT_SECRET must be set"))?,
            auth_service_url: std::env::var("AUTH_SERVICE_URL")
                .unwrap_or_else(|_| "http://auth-service:50051".to_string()),
            web_api_service_url: std::env::var("WEB_API_SERVICE_URL")
                .unwrap_or_else(|_| "http://web-api:50052".to_string()),
        })
    }
}
