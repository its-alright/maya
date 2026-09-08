mod proxy;

use axum::{
    routing::{get, post},
    Router,
};
use crate::Config;

pub fn create_routes(config: Config) -> Router {
    Router::new()
        .route("/auth/login", post(proxy::login))
        .route("/auth/register", post(proxy::register))
        .route("/auth/me", get(proxy::get_user))
        .route("/items/:id", get(proxy::get_item))
        .route("/items", post(proxy::create_item))
        .route("/items", get(proxy::get_items))
        .with_state(config)
}