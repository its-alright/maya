use crate::config::Config;
use crate::models::item::Item;
use crate::repository::item_repo::ItemRepository;
use anyhow::Result;
use std::sync::Arc;
use tracing::{error, info, instrument};
use uuid::Uuid;

pub struct WebApiService {
    repo: Arc<ItemRepository>,
    config: Arc<Config>,
}

impl WebApiService {
    pub fn new(repo: Arc<ItemRepository>, config: Arc<Config>) -> Self {
        Self { repo, config }
    }

    #[instrument(skip(self))]
    pub async fn get_item(&self, id: String) -> Result<Option<Item>, String> {
        let uuid = Uuid::parse_str(&id).map_err(|e| {
            error!("Invalid UUID: {}", e);
            "Invalid ID format".to_string()
        })?;

        match self.repo.find_by_id(&uuid).await {
            Ok(item) => {
                info!("Found item: {:?}", item);
                Ok(item)
            }
            Err(e) => {
                error!("Database error: {}", e);
                Err("Database error".to_string())
            }
        }
    }

    #[instrument(skip(self))]
    pub async fn create_item(&self, name: String) -> Result<Item, String> {
        match self.repo.create_item(&name).await {
            Ok(item) => {
                info!("Created item: {:?}", item);
                Ok(item)
            }
            Err(e) => {
                error!("Failed to create item: {}", e);
                Err("Failed to create item".to_string())
            }
        }
    }

    #[instrument(skip(self))]
    pub async fn list_items(&self, limit: i64, offset: i64) -> Result<Vec<Item>, String> {
        match self.repo.list_items(limit, offset).await {
            Ok(items) => {
                info!("Listed {} items", items.len());
                Ok(items)
            }
            Err(e) => {
                error!("Database error: {}", e);
                Err("Database error".to_string())
            }
        }
    }
}

pub async fn run_server(config: Config, pool: sqlx::PgPool) -> Result<()> {
    let port = config.port;
    let repo = Arc::new(ItemRepository::new(pool));
    let _ = WebApiService::new(repo, Arc::new(config));

    info!("Web API gRPC server starting on port {}", port);
    let _: std::net::SocketAddr = format!("0.0.0.0:{}", port).parse()?;

    // TODO: Реализовать gRPC сервер с использованием tonic
    // Для демонстрации пока просто логируем

    Ok(())
}
