use std::sync::Arc;
use anyhow::Result;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{info, error, instrument};
use jsonwebtoken::{encode, EncodingKey, Header};
use chrono::{Utc, Duration};
use serde::{Serialize, Deserialize};

use crate::{
    config::Config,
    repository::user_repo::UserRepository,
    models::user::{User, AuthResponse, LoginRequest, CreateUserRequest},
};

#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    sub: String,
    exp: usize,
    iat: usize,
    user_id: String,
    email: String,
    name: String,
}

pub struct AuthService {
    repo: Arc<UserRepository>,
    config: Arc<Config>,
}

impl AuthService {
    pub fn new(repo: Arc<UserRepository>, config: Arc<Config>) -> Self {
        Self { repo, config }
    }

    #[instrument(skip(self))]
    pub async fn register(&self, request: CreateUserRequest) -> Result<User, Status> {
        info!("Registering user: {}", request.email);

        // Проверка существования пользователя
        if let Some(_) = self.repo.find_by_email(&request.email).await.map_err(|e| {
            error!("Database error: {}", e);
            Status::internal("Database error")
        })? {
            return Err(Status::already_exists("User already exists"));
        }

        let user = self.repo
            .create_user(&request.email, &request.password, &request.name)
            .await
            .map_err(|e| {
                error!("Failed to create user: {}", e);
                Status::internal("Failed to create user")
            })?;

        Ok(user)
    }

    #[instrument(skip(self))]
    pub async fn login(&self, request: LoginRequest) -> Result<AuthResponse, Status> {
        info!("Login attempt: {}", request.email);

        let user = self.repo
            .find_by_email(&request.email)
            .await
            .map_err(|e| {
                error!("Database error: {}", e);
                Status::internal("Database error")
            })?
            .ok_or_else(|| {
                error!("User not found: {}", request.email);
                Status::not_found("User not found")
            })?;

        // Проверка пароля
        let is_valid = self.repo
            .verify_password(&user, &request.password)
            .await
            .map_err(|e| {
                error!("Password verification error: {}", e);
                Status::internal("Password verification error")
            })?;

        if !is_valid {
            error!("Invalid password for user: {}", request.email);
            return Err(Status::unauthenticated("Invalid credentials"));
        }

        // Генерация JWT
        let token = self.generate_jwt(&user)?;

        Ok(AuthResponse {
            token,
            user_id: user.id,
            email: user.email,
            name: user.name,
        })
    }

    #[instrument(skip(self))]
    pub async fn get_user(&self, user_id: String) -> Result<User, Status> {
        let user_id = uuid::Uuid::parse_str(&user_id).map_err(|_| {
            error!("Invalid UUID format: {}", user_id);
            Status::invalid_argument("Invalid user ID format")
        })?;

        let user = self.repo
            .find_by_id(&user_id)
            .await
            .map_err(|e| {
                error!("Database error: {}", e);
                Status::internal("Database error")
            })?
            .ok_or_else(|| {
                error!("User not found: {}", user_id);
                Status::not_found("User not found")
            })?;

        Ok(user)
    }

    fn generate_jwt(&self, user: &User) -> Result<String, Status> {
        let now = Utc::now();
        let exp = now + Duration::hours(self.config.jwt_expiration_hours as i64);

        let claims = JwtClaims {
            sub: user.id.to_string(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            user_id: user.id.to_string(),
            email: user.email.clone(),
            name: user.name.clone(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )
            .map_err(|e| {
                error!("Failed to generate JWT: {}", e);
                Status::internal("Failed to generate token")
            })
    }
}

pub async fn run_server(config: Config, pool: sqlx::PgPool) -> Result<()> {
    let repo = Arc::new(UserRepository::new(pool));
    let auth_service = AuthService::new(repo, Arc::new(config));

    // Здесь будет gRPC сервер с использованием tonic
    info!("Auth gRPC server starting on port {}", config.port);
    let addr = format!("0.0.0.0:{}", config.port).parse()?;

    // TODO: Реализовать gRPC сервис с использованием tonic
    // Для демонстрации пока просто логируем

    Ok(())
}