#![allow(dead_code)]

mod routes;

use anyhow::Result;
use axum::{Router, http::HeaderValue};
use dotenvy::dotenv;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use opentelemetry::KeyValue;
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions::resource::{
    DEPLOYMENT_ENVIRONMENT_NAME, SERVICE_NAME, SERVICE_VERSION,
};
use shared::config::Config;
use shared::middleware::metrics::MetricsMiddleware;

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
        .unwrap_or_else(|_| EnvFilter::new("api-gateway=info,tower_http=info,info"));

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
        "Service api-gateway starting, Port: 8080, Environment: {}",
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
        .nest("/api", routes::create_routes(config.clone()))
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

    info!("Service api-gateway stopped");
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
