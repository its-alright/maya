use crate::handlers::state::AppState;
use axum::response::IntoResponse;
use axum::{
    Router,
    routing::{get, post},
};
use shared::auth::user::AuthUser;

pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/register", post(register))
        .route("/auth/me", get(get_user))
        .with_state(state)
}

async fn login() -> impl IntoResponse {}
async fn register() -> impl IntoResponse {}
async fn get_user(user: AuthUser) -> impl IntoResponse {}
