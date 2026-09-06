use anyhow::Result;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub port: u16,
    pub environment: String,
    pub cors_origin: String,
    pub otel_endpoint: String,
    pub jwt_secret: String,
    pub auth_service_url: String,
    pub web_api_service_url: String,
    pub rate_limit_requests: u32,
    pub rate_limit_period_secs: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            port: std::env::var("GATEWAY_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()?,
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            cors_origin: std::env::var("CORS_ORIGIN").unwrap_or_else(|_| "*".to_string()),
            otel_endpoint: std::env::var("OTEL_ENDPOINT")
                .unwrap_or_else(|_| "http://openobserve:5080".to_string()),
            jwt_secret: std::env::var("JWT_SECRET")
                .map_err(|_| anyhow::anyhow!("JWT_SECRET must be set"))?,
            auth_service_url: std::env::var("AUTH_SERVICE_URL")
                .unwrap_or_else(|_| "http://auth-service:50051".to_string()),
            web_api_service_url: std::env::var("WEB_API_SERVICE_URL")
                .unwrap_or_else(|_| "http://web-api:50052".to_string()),
            rate_limit_requests: std::env::var("RATE_LIMIT_REQUESTS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()?,
            rate_limit_period_secs: std::env::var("RATE_LIMIT_PERIOD_SECS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()?,
        })
    }
}