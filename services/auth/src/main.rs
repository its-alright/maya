#![allow(dead_code)]

mod routes;

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
use opentelemetry_semantic_conventions::{
    SCHEMA_URL,
    resource::{DEPLOYMENT_ENVIRONMENT, SERVICE_NAME, SERVICE_VERSION},
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Service auth start");
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

    let _guard = init_telemetry(&config, resource(), log_filter)?;

    info!(
        "Service auth starting, Port: 8080, Environment: {}",
        config.environment
    );

    // 6. CORS настройки
    let cors = CorsLayer::new()
        .allow_origin(config.cors_origin.parse::<HeaderValue>()?)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
        ])
        .allow_headers(vec![
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ]);

    // 7. Создаем роутер с метриками
    let metrics_middleware = Arc::new(MetricsMiddleware::new());

    let app = Router::new()
        .nest("/api", routes::create_routes())
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
    info!("Http server auth listening on http://{}", addr);
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
    Resource::from_schema_url(
        [
            KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
            KeyValue::new(DEPLOYMENT_ENVIRONMENT, "develop"),
        ],
        SCHEMA_URL,
    )
}
