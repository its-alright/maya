use sqlx::PgPool;
use anyhow::Result;
use uuid::Uuid;
use crate::models::user::User;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_user(
        &self,
        email: &str,
        password: &str,
        name: &str,
    ) -> Result<User> {
        // Хеширование пароля
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)?
            .to_string();

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (email, name, password_hash)
            VALUES ($1, $2, $3)
            RETURNING id, email, name, password_hash, created_at, updated_at
            "#
        )
            .bind(email)
            .bind(name)
            .bind(password_hash)
            .fetch_one(&self.pool)
            .await?;

        Ok(user)
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, name, password_hash, created_at, updated_at
            FROM users
            WHERE email = $1
            "#
        )
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;

        Ok(user)
    }

    pub async fn verify_password(&self, user: &User, password: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(&user.password_hash)?;
        Ok(Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }

    pub async fn find_by_id(&self, id: &Uuid) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, name, password_hash, created_at, updated_at
            FROM users
            WHERE id = $1
            "#
        )
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(user)
    }
}