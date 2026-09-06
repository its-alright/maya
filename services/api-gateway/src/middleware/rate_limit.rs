use tower_http::limit::RateLimitLayer;
use std::time::Duration;

pub fn rate_limit_layer() -> RateLimitLayer {
    // 100 запросов в минуту
    RateLimitLayer::new(100, Duration::from_secs(60))
}