use sqlx::PgPool;
use anyhow::Result;
use tracing::info;
use uuid::Uuid;
use crate::models::item::Item;

pub struct ItemRepository {
    pool: PgPool,
}

impl ItemRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_item(&self, name: &str) -> Result<Item> {
        let item = sqlx::query_as::<_, Item>(
            r#"
            INSERT INTO items (name)
            VALUES ($1)
            RETURNING id, name, created_at, updated_at
            "#
        )
            .bind(name)
            .fetch_one(&self.pool)
            .await?;

        Ok(item)
    }

    pub async fn find_by_id(&self, id: &Uuid) -> Result<Option<Item>> {
        let item = sqlx::query_as::<_, Item>(
            r#"
            SELECT id, name, created_at, updated_at
            FROM items
            WHERE id = $1
            "#
        )
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(item)
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_items(&self, limit: i64, offset: i64) -> Result<Vec<Item>> {
        info!("start loading items");
        let items = sqlx::query_as::<_, Item>(
            r#"
            SELECT id, name, created_at, updated_at
            FROM items
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#
        )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        info!("end loading items");

        Ok(items)
    }

    pub async fn update_item(&self, id: &Uuid, name: &str) -> Result<Option<Item>> {
        let item = sqlx::query_as::<_, Item>(
            r#"
            UPDATE items
            SET name = $1, updated_at = now()
            WHERE id = $2
            RETURNING id, name, created_at, updated_at
            "#
        )
            .bind(name)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(item)
    }

    pub async fn delete_item(&self, id: &Uuid) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM items
            WHERE id = $1
            "#
        )
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}