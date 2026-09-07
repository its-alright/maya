use crate::models::item::{CreateItemRequest, ItemResponse, UpdateItemRequest};
use crate::repository::item_repo::ItemRepository;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

pub type AppState = Arc<ItemRepository>;

pub fn item_routes(state: AppState) -> axum::Router {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/", post(create_item).get(list_items))
        .route("/:id", get(get_item).put(update_item).delete(delete_item))
        .with_state(state)
}

pub async fn create_item(
    State(repo): State<AppState>,
    Json(payload): Json<CreateItemRequest>,
) -> impl IntoResponse {
    info!("Creating item: name={}", payload.name);

    match repo.create_item(&payload.name).await {
        Ok(item) => {
            let response: ItemResponse = item.into();
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            error!("Failed to create item: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to create item" })),
            )
                .into_response()
        }
    }
}

#[tracing::instrument(skip(repo))]
pub async fn list_items(State(repo): State<AppState>) -> impl IntoResponse {
    info!("Listing items");

    match repo.list_items(100, 0).await {
        Ok(items) => {
            info!("items loaded");
            let responses: Vec<ItemResponse> = items.into_iter().map(|item| item.into()).collect();
            (StatusCode::OK, Json(responses)).into_response()
        }
        Err(e) => {
            error!("Failed to list items: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to list items" })),
            )
                .into_response()
        }
    }
}

pub async fn get_item(State(repo): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    info!("Getting item: id={}", id);

    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(e) => {
            error!("Invalid UUID: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid ID format" })),
            )
                .into_response();
        }
    };

    match repo.find_by_id(&uuid).await {
        Ok(Some(item)) => {
            let response: ItemResponse = item.into();
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(None) => {
            error!("Item not found: {}", id);
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Item not found" })),
            )
                .into_response()
        }
        Err(e) => {
            error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Database error" })),
            )
                .into_response()
        }
    }
}

pub async fn update_item(
    State(repo): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateItemRequest>,
) -> impl IntoResponse {
    info!("Updating item: id={}, name={}", id, payload.name);

    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(e) => {
            error!("Invalid UUID: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid ID format" })),
            )
                .into_response();
        }
    };

    match repo.update_item(&uuid, &payload.name).await {
        Ok(Some(item)) => {
            let response: ItemResponse = item.into();
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(None) => {
            error!("Item not found: {}", id);
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Item not found" })),
            )
                .into_response()
        }
        Err(e) => {
            error!("Failed to update item: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to update item" })),
            )
                .into_response()
        }
    }
}

pub async fn delete_item(
    State(repo): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    info!("Deleting item: id={}", id);

    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(e) => {
            error!("Invalid UUID: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid ID format" })),
            )
                .into_response();
        }
    };

    match repo.delete_item(&uuid).await {
        Ok(true) => (StatusCode::NO_CONTENT, Json(serde_json::json!({}))).into_response(),
        Ok(false) => {
            error!("Item not found: {}", id);
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Item not found" })),
            )
                .into_response()
        }
        Err(e) => {
            error!("Failed to delete item: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to delete item" })),
            )
                .into_response()
        }
    }
}
