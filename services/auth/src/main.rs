#![allow(dead_code)]

mod handlers;
mod models;
mod repository;
mod services;

use anyhow::Result;
use axum::{Router, http::HeaderValue};
use dotenvy::dotenv;
use shared::{config::Config, middleware::metrics::MetricsMiddleware};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use crate::handlers::state::AppState;
use opentelemetry::KeyValue;
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions::resource::{
    DEPLOYMENT_ENVIRONMENT_NAME, SERVICE_NAME, SERVICE_VERSION,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Service start");
    std::panic::set_hook(Box::new(|info| {
        eprintln!("PANIC: {info}");
    }));

    // 1. Загрузка .env
    dotenv().ok();

    // 2. Загрузка конфигурации
    let config = Config::from_env()?;

    // 3. Инициализация telemetry
    let log_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("auth=info,tower_http=info,info"));

    eprintln!("DEBUG: before init_telemetry");

    let _guard = match shared::otel::init_telemetry(&config, resource(), log_filter) {
        Ok(g) => {
            eprintln!("DEBUG: init_telemetry OK");
            g
        }
        Err(e) => {
            eprintln!("DEBUG: init_telemetry FAILED: {e:#}");
            return Ok(()); // или std::process::exit(1)
        }
    };

    eprintln!("DEBUG: after init_telemetry");

    info!(
        "Service auth starting, Port: 8080, Environment: {}",
        config.environment
    );

    // 6. CORS настройки
    let cors = CorsLayer::new()
        .allow_origin(config.cors_origin.parse::<HeaderValue>()?)
        .allow_methods([
            http::Method::GET,
            http::Method::POST,
            http::Method::PUT,
            http::Method::DELETE,
        ])
        .allow_headers(vec![
            http::header::AUTHORIZATION,
            http::header::CONTENT_TYPE,
        ]);

    info!("Connecting to database");
    let pool = repository::create_pool(&config.database_url).await?;

    info!("Run migrations");

    sqlx::migrate!()
        .set_ignore_missing(true)
        .run(&pool)
        .await
        .expect("Failed to run migrations for service-a");

    info!("Database initialized successfully");

    let repo = repository::users_repo::UsersRepository::new(pool);
    let users_service = Arc::new(services::users::UsersService::new(repo));

    // 7. Создаем роутер с метриками
    let metrics_middleware = Arc::new(MetricsMiddleware::new());

    let cfg = Arc::new(config);
    let app_state: handlers::state::AppState = AppState::new(cfg, users_service);

    let app = Router::new()
        .nest("/api", handlers::create_routes(app_state))
        .layer(cors)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(axum::middleware::from_fn({
            let middleware = metrics_middleware.clone();
            move |req, next| {
                let middleware = middleware.clone();
                async move { middleware.layer(req, next).await }
            }
        }));

    // 8. Запускаем сервер
    let addr = "0.0.0.0:8080";
    info!("Http server auth listening on {}", addr);
    info!("Press Ctrl+C to stop");

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Запускаем сервер в отдельной задаче
    let server_task = tokio::spawn(async { axum::serve(listener, app).await });

    // Ожидаем сигнал завершения
    tokio::select! {
        result = server_task => {
            match result {
                Ok(Ok(_)) => info!("Http server auth stopped normally"),
                Ok(Err(e)) => error!("Http server auth error: {}", e),
                Err(e) => error!("Http server auth task error: {}", e),
            }
        }
        _ = tokio::signal::ctrl_c() => {
             info!("Shutdown signal received, stopping...");
        }
    }

    info!("Service auth stopped");
    Ok(())
}

fn resource() -> Resource {
    Resource::builder()
        .with_service_name(env!("CARGO_PKG_NAME"))
        .with_attributes([
            KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
            KeyValue::new(DEPLOYMENT_ENVIRONMENT_NAME, "develop"),
        ])
        .build()
}
