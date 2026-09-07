mod config;
mod handlers;
mod models;
mod repository;
mod metrics;

use crate::metrics::middleware::MetricsMiddleware;

use anyhow::Result;
use axum::{Router, http::HeaderValue};
use config::Config;
use dotenvy::dotenv;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use opentelemetry::{Key, KeyValue, global, trace::TracerProvider};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    Resource,
    metrics::{
        Aggregation, Instrument, MeterProviderBuilder, PeriodicReader, SdkMeterProvider, Stream,
        reader::{DefaultAggregationSelector, DefaultTemporalitySelector},
    },
    runtime,
    trace::{BatchConfig, RandomIdGenerator, Sampler, Tracer},
};
use opentelemetry_semantic_conventions::{
    SCHEMA_URL,
    resource::{DEPLOYMENT_ENVIRONMENT, SERVICE_NAME, SERVICE_VERSION},
};
use std::io::Error;
use tonic::metadata::*;
use tracing_core::Level;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Загрузка .env
    dotenv().ok();

    // 2. Настройка логирования
    tracing::info!(
        monotonic_counter.main = 1_u64,
        key_1 = "bar",
        key_2 = 10,
        "handle foo",
    );

    otl_metadata().expect("failed init otl metadata");
    init_tracing_subscriber();
    init_tracer();
    init_meter_provider();

    info!("========================================");
    info!("Web-API Service starting...");
    info!("========================================");
    foo().await;

    info!("foo executed...");

    // 3. Загрузка конфигурации
    let config = Config::from_env()?;
    info!("Port: {}", config.port);
    info!("Environment: {}", config.environment);

    info!("db url: {}", config.database_url.clone());

    // 4. Инициализация БД
    info!("Connecting to database...");
    let pool = repository::create_pool(&config.database_url).await?;
    repository::run_migrations(&pool).await?;
    info!("Database initialized successfully");

    // 5. Создаем репозиторий
    let repo = Arc::new(repository::item_repo::ItemRepository::new(pool));

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

    // 7. Создаем роутер
    let app = Router::new()
        .nest("/api/items", handlers::item_routes(repo.clone()))
        .layer(cors)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    // 8. Запускаем сервер
    let addr = format!("0.0.0.0:{}", config.port);
    info!("Web API HTTP server listening on http://{}", addr);
    info!("Press Ctrl+C to stop");

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Запускаем сервер в отдельной задаче
    let server_task = tokio::spawn(async { axum::serve(listener, app).await });

    // Ожидаем сигнал завершения
    tokio::select! {
        result = server_task => {
            match result {
                Ok(Ok(_)) => info!("Server stopped normally"),
                Ok(Err(e)) => error!("Server error: {}", e),
                 Err(e) => error!("Server task error: {}", e),
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
    Resource::from_schema_url(
        [
            KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
            KeyValue::new(DEPLOYMENT_ENVIRONMENT, "develop"),
        ],
        SCHEMA_URL,
    )
}

fn otl_metadata() -> Result<MetadataMap, Error> {
    let otel_email =
        std::env::var("ZO_ROOT_USER_EMAIL").unwrap_or_else(|_| "admin@example.com".to_string());

    let otel_password =
        std::env::var("ZO_ROOT_USER_PASSWORD").unwrap_or_else(|_| "admin123".to_string());

    let auth_string = format!("{}:{}", otel_email, otel_password);
    let base64_token = base64::encode(auth_string.clone());

    info!("auth_string1 {}", auth_string.clone());
    info!("base64_token1 {}", base64_token.clone());
    let auth_header_value = format!("basic {}", base64_token.clone());

    let mut map = MetadataMap::with_capacity(3);
    map.insert("authorization", auth_header_value.parse().unwrap());
    map.insert("organization", "default".parse().unwrap());
    map.insert("stream-name", "default".parse().unwrap());
    Ok(map)
}

fn init_meter_provider() -> SdkMeterProvider {
    let endpoint =
        std::env::var("OTEL_ENDPOINT").unwrap_or_else(|_| "http://openobserve:5081".to_string());

    info!("TRACE endpoint {}", endpoint);

    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint)
        .with_protocol(opentelemetry_otlp::Protocol::Grpc)
        .with_metadata(otl_metadata().unwrap())
        .build_metrics_exporter(
            Box::new(DefaultAggregationSelector::new()),
            Box::new(DefaultTemporalitySelector::new()),
        )
        .unwrap();
    let reader = PeriodicReader::builder(exporter, runtime::Tokio)
        .with_interval(std::time::Duration::from_secs(30))
        .build();
    // For debugging in development
    let stdout_reader = PeriodicReader::builder(
        opentelemetry_stdout::MetricsExporter::default(),
        runtime::Tokio,
    )
    .build();
    // Rename foo metrics to foo_named and drop key_2 attribute
    let view_foo = |instrument: &Instrument| -> Option<Stream> {
        if instrument.name == "foo" {
            Some(
                Stream::new()
                    .name("foo_named")
                    .allowed_attribute_keys([Key::from("key_1")]),
            )
        } else {
            None
        }
    };
    // Set Custom histogram boundaries for baz metrics
    let view_baz = |instrument: &Instrument| -> Option<Stream> {
        if instrument.name == "baz" {
            Some(
                Stream::new()
                    .name("baz")
                    .aggregation(Aggregation::ExplicitBucketHistogram {
                        boundaries: vec![0.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0],
                        record_min_max: true,
                    }),
            )
        } else {
            None
        }
    };
    let meter_provider = MeterProviderBuilder::default()
        .with_resource(resource())
        .with_reader(reader)
        .with_reader(stdout_reader)
        .with_view(view_foo)
        .with_view(view_baz)
        .build();
    global::set_meter_provider(meter_provider.clone());
    meter_provider
}

fn init_tracer() -> Tracer {
    let endpoint =
        std::env::var("OTEL_ENDPOINT").unwrap_or_else(|_| "http://localhost:5081".to_string());

    info!("TRACE endpoint {}", endpoint);

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                // Customize sampling strategy
                .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                    1.0,
                ))))
                // If export trace to AWS X-Ray, you can use XrayIdGenerator
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource()),
        )
        .with_batch_config(BatchConfig::default())
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint)
                .with_metadata(otl_metadata().unwrap()),
        )
        .install_batch(runtime::Tokio)
        .unwrap();
    global::set_tracer_provider(provider.clone());
    provider.tracer("tracing-otel-subscriber")
}

fn init_tracing_subscriber() -> OtelGuard {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("web_api=info,tower_http=info,info"));

    let meter_provider = init_meter_provider();
    let tracer = init_tracer();

    let otel_log_layer = tracing_opentelemetry::OpenTelemetryLayer::new(tracer);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().json())
        .with(otel_log_layer)
        .with(MetricsLayer::new(meter_provider.clone()))
        .init();
    OtelGuard { meter_provider }
}

struct OtelGuard {
    meter_provider: SdkMeterProvider,
}
impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Err(err) = self.meter_provider.shutdown() {
            eprintln!("{err:?}");
        }
        opentelemetry::global::shutdown_tracer_provider();
    }
}

#[tracing::instrument]
async fn foo() {
    tracing::info!(
        monotonic_counter.foo = 1_u64,
        key_1 = "bar",
        key_2 = 10,
        "handle foo",
    );
    tracing::info!(histogram.baz = 10, "histogram example",);
}
