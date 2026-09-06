use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use crate::Config;

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

// Прокси для логина
pub async fn login(
    State(config): State<Config>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    info!("Login request for user: {}", payload.email);

    // Здесь делаем gRPC вызов к auth-service
    // Для упрощения пока возвращаем заглушку
    let response = ApiResponse {
        success: true,
        data: Some(serde_json::json!({
            "token": "dummy_jwt_token",
            "user": {
                "id": "123",
                "email": payload.email,
                "name": "Test User"
            }
        })),
        error: None,
    };

    (StatusCode::OK, Json(response))
}

// Прокси для регистрации
pub async fn register(
    State(config): State<Config>,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    info!("Register request for user: {}", payload.email);

    let response = ApiResponse {
        success: true,
        data: Some(serde_json::json!({
            "id": "uuid",
            "email": payload.email,
            "name": payload.name
        })),
        error: None,
    };

    (StatusCode::CREATED, Json(response))
}

// Прокси для получения пользователя
pub async fn get_user(headers: HeaderMap) -> impl IntoResponse {
    // Здесь проверяем JWT и получаем пользователя из auth-service
    info!("Get user request");

    let response = ApiResponse {
        success: true,
        data: Some(serde_json::json!({
            "id": "123",
            "email": "user@example.com",
            "name": "Test User"
        })),
        error: None,
    };

    (StatusCode::OK, Json(response))
}

// Прокси для получения item из web-api
pub async fn get_item(
    Path(id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    info!("Get item request: {}", id);

    // Здесь делаем gRPC вызов к web-api с передачей JWT
    let response = ApiResponse {
        success: true,
        data: Some(serde_json::json!({
            "id": id,
            "name": format!("Item {}", id)
        })),
        error: None,
    };

    (StatusCode::OK, Json(response))
}

// Прокси для создания item
pub async fn create_item(
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    info!("Create item request");

    let response = ApiResponse {
        success: true,
        data: Some(serde_json::json!({
            "id": "new-uuid",
            "name": payload.get("name").unwrap_or(&serde_json::json!("Default Name"))
        })),
        error: None,
    };

    (StatusCode::CREATED, Json(response))
}