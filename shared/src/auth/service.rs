use crate::auth::user::{AuthError, Claims};
use jsonwebtoken::{DecodingKey, Validation, decode};
use std::sync::Arc;
use tracing::error;

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
