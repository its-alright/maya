use axum::response::IntoResponse;
use axum::{
    Router,
    routing::{get, post},
};
use http::HeaderMap;

pub fn create_routes() -> Router {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/register", post(register))
        .route("/auth/me", get(get_user))
    //.with_state(config)
}

async fn login(headers: HeaderMap) -> impl IntoResponse {}
async fn register(headers: HeaderMap) -> impl IntoResponse {}
async fn get_user(headers: HeaderMap) -> impl IntoResponse {}
