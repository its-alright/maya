pub mod item_repo;

use sqlx::postgres::{PgPoolOptions, PgPool};
use anyhow::Result;

pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    Ok(PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?)
}