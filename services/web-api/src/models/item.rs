use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Item {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateItemRequest {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateItemRequest {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemResponse {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

// Конвертация Item в ItemResponse
impl From<Item> for ItemResponse {
    fn from(item: Item) -> Self {
        Self {
            id: item.id.to_string(),
            name: item.name,
            created_at: item.created_at.to_rfc3339(),
            updated_at: item.updated_at.to_rfc3339(),
        }
    }
}