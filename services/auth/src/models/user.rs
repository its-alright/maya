use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    id: Uuid,
    email: String,
    display_name: String,
    hashed_password: String,
    salt: String,
    created_at: DateTime<Utc>,
}
