mod proxy;

use crate::Config;
use axum::{
    Router,
    routing::{get, post},
};

pub fn create_routes(config: Config) -> Router {
    Router::new()
        // Auth routes
        .route("/auth/login", post(proxy::login))
        .route("/auth/register", post(proxy::register))
        .route("/auth/me", get(proxy::get_user))
        // Items routes (прокси в web-api)
        .route("/items", get(proxy::get_items).post(proxy::create_item))
        .route("/items/:id", get(proxy::get_item))
        .with_state(config)
}
