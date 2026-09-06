use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub user_id: String,
    pub email: String,
}

pub struct AuthService {
    jwt_secret: Arc<String>,
}

impl AuthService {
    pub fn new(jwt_secret: String) -> Self {
        Self {
            jwt_secret: Arc::new(jwt_secret),
        }
    }

    pub async fn validate_token(&self, token: &str) -> Result<Claims, AuthError> {
        let validation = Validation::default();
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
            .map_err(|e| {
                error!("JWT validation failed: {}", e);
                AuthError::InvalidToken
            })?;

        Ok(token_data.claims)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid token")]
    InvalidToken,
    #[error("Missing authorization header")]
    MissingHeader,
}

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