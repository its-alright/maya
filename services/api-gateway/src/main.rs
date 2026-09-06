mod config;
mod middleware;
mod routes;

use anyhow::Result;
use axum::{http::HeaderValue, Router};
use config::Config;
use dotenvy::dotenv;
use opentelemetry::KeyValue;
use opentelemetry_otlp::SpanExporter;
use opentelemetry_sdk::{trace::Config as TraceConfig, Resource};
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let config = Config::from_env()?;

    // Инициализация telemetry
    init_telemetry(&config).await?;

    info!("Starting API Gateway on port {}", config.port);

    // CORS настройки
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

    // Создание роутера с middleware
    let app = Router::new()
        .nest("/api", routes::create_routes(config.clone()))
        .layer(cors)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(middleware::logging::request_logging_layer());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}

async fn init_telemetry(config: &Config) -> Result<()> {
    let endpoint = config.otel_endpoint.to_string();

    // Создаем ресурс с сервисной информацией
    let resource = Resource::new(vec![
        KeyValue::new("service.name", "api-gateway"),
        KeyValue::new("service.namespace", "microservices"),
        KeyValue::new("deployment.environment", config.environment.clone()),
    ]);

    // Настройка OTLP exporter
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;

    // Настройка trace provider
    let trace_config = TraceConfig::default()
        .with_resource(resource)
        .with_sampler(opentelemetry_sdk::trace::Sampler::AlwaysOn);

    let tracer_provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_config(trace_config)
        .build();

    // Устанавливаем глобальный провайдер
    let _ = opentelemetry::global::set_tracer_provider(tracer_provider.clone());

    // Создаем OTLP слой для tracing
    let otel_layer = OpenTelemetryLayer::new(tracer_provider);

    // Настройка subscriber
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("api_gateway=info,tower_http=info,info"));

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().json())
        .with(env_filter)
        .with(otel_layer)
        .try_init()?;

    Ok(())
}