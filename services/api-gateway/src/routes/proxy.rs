use hex;
use opentelemetry::trace::TraceContextExt;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::Config;
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

static CLIENT: Lazy<Client> = Lazy::new(Client::new);

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

#[tracing::instrument]
async fn proxy_response(client: Client, request: reqwest::RequestBuilder) -> impl IntoResponse {
    info!("Send downstream request");

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            info!("Get downstream response successful, status {}", status);
            let body = response.text().await.unwrap_or_default();
            (status, body).into_response()
        }
        Err(e) => {
            error!("Get downstream response error: {}", e);
            (StatusCode::BAD_GATEWAY, format!("Bad gateway: {}", e)).into_response()
        }
    }
}

// Прокси для логина
#[tracing::instrument]
pub async fn login(
    State(config): State<Config>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    info!("Login request for user: {}", payload.email);

    let client = Client::new();
    match client
        .post(&format!("{}/login", config.auth_service_url))
        .json(&payload)
        .send()
        .await
    {
        Ok(response) => match response.json::<serde_json::Value>().await {
            Ok(data) => {
                info!("Login successful for: {}", payload.email);
                (StatusCode::OK, Json(data)).into_response()
            }
            Err(e) => {
                error!("Failed to parse login response: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "success": false,
                        "error": "Invalid response from auth service"
                    })),
                )
                    .into_response()
            }
        },
        Err(e) => {
            error!("Auth service error: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "success": false,
                    "error": "Auth service unavailable"
                })),
            )
                .into_response()
        }
    }
}

// Прокси для регистрации
#[tracing::instrument]
pub async fn register(
    State(config): State<Config>,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    info!("Register request for user: {}", payload.email);

    let client = Client::new();
    match client
        .post(&format!("{}/register", config.auth_service_url))
        .json(&payload)
        .send()
        .await
    {
        Ok(response) => match response.json::<serde_json::Value>().await {
            Ok(data) => {
                info!("Register successful for: {}", payload.email);
                (StatusCode::CREATED, Json(data)).into_response()
            }
            Err(e) => {
                error!("Failed to parse register response: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "success": false,
                        "error": "Invalid response from auth service"
                    })),
                )
                    .into_response()
            }
        },
        Err(e) => {
            error!("Auth service error: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "success": false,
                    "error": "Auth service unavailable"
                })),
            )
                .into_response()
        }
    }
}

// Прокси для получения пользователя
pub async fn get_user(_headers: HeaderMap) -> impl IntoResponse {
    // Здесь проверяем JWT и получаем пользователя из auth-service
    info!("Get user request");

    // Здесь делаем http вызов к auth-service (пока возвращаем заглушку)
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
pub async fn get_item(Path(id): Path<String>, _headers: HeaderMap) -> impl IntoResponse {
    info!("Get item request: {}", id);

    // Здесь делаем http вызов к web-api с передачей JWT (пока возвращаем заглушку)
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

#[tracing::instrument(skip(config))]
pub async fn get_items(headers: HeaderMap, State(config): State<Config>) -> impl IntoResponse {
    info!("Get items start");
    let downstream_endpoint = format!("{}/api/items", config.web_api_service_url);
    let mut request = CLIENT.get(downstream_endpoint);
    if let Some(auth) = headers.get("Authorization") {
        request = request.header("Authorization", auth);
    }

    let current_span = Span::current();
    let cx: opentelemetry::Context = current_span.context();
    let span_context = cx.span();
    let trace_id_hex = hex::encode(span_context.span_context().trace_id().to_bytes());
    let span_id_hex = hex::encode(span_context.span_context().span_id().to_bytes());

    let mut traceparent = String::with_capacity(55);
    traceparent.push_str("00-");
    traceparent.push_str(&trace_id_hex);
    traceparent.push_str("-");
    traceparent.push_str(&span_id_hex);
    traceparent.push_str("-01");

    request = request.header("traceparent", traceparent);

    info!("Set traceparent");

    let resp = proxy_response(CLIENT.clone(), request).await;

    info!("Get items complete");

    resp
}

// Прокси для создания item
pub async fn create_item(
    _headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    info!("Create item request");

    // Здесь делаем http вызов к web-api с передачей JWT (пока возвращаем заглушку)
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
