mod item;

use crate::handlers::item::{AppState, item_routes};

pub fn create_routes(state: AppState) -> axum::Router {
    item_routes(state)
}
