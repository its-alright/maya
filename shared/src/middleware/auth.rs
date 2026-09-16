use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};
use crate::auth::AuthService;

pub async fn auth_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Извлекаем токен из заголовка
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        warn!("Invalid authorization header format");
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..];

    // Валидация токена
    let jwt_secret = std::env::var("JWT_SECRET").map_err(|_| {
        error!("JWT_SECRET not configured");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let auth_service = AuthService::new(jwt_secret);
    let claims = auth_service
        .validate_token(token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    info!("User authenticated: {}", claims.email);

    // Добавляем claims в расширения запроса для дальнейшего использования
    let mut request = request;
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}
