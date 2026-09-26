#![allow(dead_code)]

mod handlers;
mod models;
mod repository;
mod middleware;

use anyhow::Result;
use axum::{Router, http::HeaderValue};
use dotenvy::dotenv;
use shared::{config::Config, middleware::metrics::MetricsMiddleware, otel::init_telemetry};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

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
        .unwrap_or_else(|_| EnvFilter::new("web-api=info,tower_http=info,info"));

    let _guard = init_telemetry(&config, resource(), log_filter)?;

    info!(
        "Service starting, Port: 8080, Environment: {}",
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
        .run(&pool)
        .await
        .expect("Failed to run migrations for service-a");

    info!("Database initialized successfully");

    let repo = Arc::new(repository::item_repo::ItemRepository::new(pool));

    // 7. Создаем роутер с метриками
    let metrics_middleware = Arc::new(MetricsMiddleware::new());

    let app = Router::new()
        .nest("/api/items", handlers::create_routes(repo.clone()))
        .layer(axum::middleware::from_fn(move |req, next| {
            middleware::extract_trace_context(req, next)
        }))
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
    info!("Http server listening on {}", addr);
    info!("Press Ctrl+C to stop");

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Запускаем сервер в отдельной задаче
    let server_task = tokio::spawn(async { axum::serve(listener, app).await });

    // Ожидаем сигнал завершения
    tokio::select! {
        result = server_task => {
            match result {
                Ok(Ok(_)) => info!("Http server stopped normally"),
                Ok(Err(e)) => error!("Http server error: {}", e),
                Err(e) => error!("Http server task error: {}", e),
            }
        }
        _ = tokio::signal::ctrl_c() => {
             info!("Shutdown signal received, stopping...");
        }
    }

    info!("Service stopped");
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
