use async_trait::async_trait;
use axum::{
    Json,
    extract::FromRequestParts,
    http::{StatusCode, header::AUTHORIZATION, request::Parts},
    response::{IntoResponse, Response},
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub role: String,
    pub exp: i64,
}

pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub role: String,
}

#[derive(Debug)]
pub enum AuthError {
    /// Нет заголовка Authorization или он не в формате "Bearer <token>"
    MissingToken,
    /// Токен не прошёл проверку подписи / истёк / невалидный
    InvalidToken,
    /// Токен валиден, но в нём нет нужных полей или sub не парсится
    InvalidClaims,
    /// Токен валиден, но у пользователя нет прав (для require_role)
    Forbidden,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::MissingToken => write!(f, "missing bearer token"),
            AuthError::InvalidToken => write!(f, "invalid or expired token"),
            AuthError::InvalidClaims => write!(f, "invalid token claims"),
            AuthError::Forbidden => write!(f, "forbidden"),
        }
    }
}

impl std::error::Error for AuthError {}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, code, msg) = match self {
            AuthError::MissingToken => (
                StatusCode::UNAUTHORIZED,
                "missing_token",
                "Authorization header is missing or malformed",
            ),
            AuthError::InvalidToken => (
                StatusCode::UNAUTHORIZED,
                "invalid_token",
                "Token is invalid or expired",
            ),
            AuthError::InvalidClaims => (
                StatusCode::UNAUTHORIZED,
                "invalid_claims",
                "Token claims are invalid",
            ),
            AuthError::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "Insufficient permissions",
            ),
        };

        (status, Json(json!({ "error": code, "message": msg }))).into_response()
    }
}

pub trait JwtSecretProvider {
    fn jwt_secret(&self) -> &[u8];
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: JwtSecretProvider + Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(AuthError::MissingToken)?;

        let data = decode::<Claims>(
            auth,
            &DecodingKey::from_secret(state.jwt_secret()),
            &Validation::new(Algorithm::HS256),
        )
        .map_err(|_| AuthError::InvalidToken)?;

        let user_id: Uuid = data
            .claims
            .sub
            .parse()
            .map_err(|_| AuthError::InvalidClaims)?;

        Ok(AuthUser {
            id: user_id,
            email: data.claims.email,
            role: data.claims.role,
        })
    }
}
