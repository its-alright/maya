use crate::services::users::UsersService;
use shared::auth::user::JwtSecretProvider;
use shared::config::Config;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>, // Arc<Config>, а не Arc<Service>
    pub service: Arc<UsersService>,
}

impl AppState {
    pub fn new(config: Arc<Config>, service: Arc<UsersService>) -> AppState {
        AppState { config, service }
    }
}

impl JwtSecretProvider for AppState {
    fn jwt_secret(&self) -> &[u8] {
        self.config.jwt_secret.as_bytes()
    }
}
