mod config;
mod handlers;
mod metrics;
mod models;
mod repository;

use anyhow::Result;
use axum::{Router, http::HeaderValue};
use config::Config;
use dotenvy::dotenv;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use opentelemetry::{KeyValue, global, trace::TracerProvider};
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
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Загрузка .env / .env.local
    dotenvy::from_filename(".env.local").ok();
    dotenv().ok();

    // 2. Загрузка конфигурации
    let config = Config::from_env()?;

    // 3. Инициализация telemetry (ОДИН РАЗ!)
    let _guard = init_telemetry(&config)?;

    info!("========================================");
    info!("Web-API Service starting...");
    info!("========================================");

    info!("Port: {}", config.port);
    info!("Environment: {}", config.environment);

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

    // 7. Создаем роутер с метриками
    let metrics_middleware = Arc::new(metrics::middleware::MetricsMiddleware::new());

    let app = Router::new()
        .nest("/api/items", handlers::item_routes(repo.clone()))
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

fn init_telemetry(config: &Config) -> Result<OtelGuard> {
    // Инициализируем метрики и трейсы один раз
    let meter_provider = init_meter_provider(config);
    let tracer = init_tracer(config);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("web_api=info,tower_http=info,info"));

    let otel_layer = OpenTelemetryLayer::new(tracer);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().json())
        .with(otel_layer)
        .with(MetricsLayer::new(meter_provider.clone()))
        .init();

    Ok(OtelGuard { meter_provider })
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

fn otl_metadata(config: &Config) -> Result<MetadataMap, Error> {
    let auth_string = format!("{}:{}", config.otel_user, config.otel_password);
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

fn init_meter_provider(config: &Config) -> SdkMeterProvider {
    let endpoint = config.otel_endpoint.as_str();
    info!("init_meter_provider endpoint {}", endpoint);

    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint)
        .with_protocol(opentelemetry_otlp::Protocol::Grpc)
        .with_metadata(otl_metadata(config).unwrap())
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

    let view_duration = |instrument: &Instrument| -> Option<Stream> {
        if instrument.name == "http_request_duration_seconds" {
            Some(
                Stream::new().aggregation(Aggregation::ExplicitBucketHistogram {
                    boundaries: vec![
                        0.0005, // 0.5ms
                        0.001,  // 1ms
                        0.002,  // 2ms
                        0.005,  // 5ms
                        0.01,   // 10ms
                        0.025,  // 25ms
                        0.05,   // 50ms
                        0.1,    // 100ms
                        0.25,   // 250ms
                        0.5,    // 500ms
                        1.0,    // 1s
                        2.5,    // 2.5s
                        5.0,    // 5s
                        10.0,   // 10s
                    ],
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
        .with_view(view_duration)
        .build();
    global::set_meter_provider(meter_provider.clone());
    meter_provider
}

fn init_tracer(config: &Config) -> Tracer {
    let endpoint = config.otel_endpoint.as_str();

    info!("init_tracer endpoint {}", endpoint);

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
                .with_metadata(otl_metadata(config).unwrap()),
        )
        .install_batch(runtime::Tokio)
        .unwrap();
    global::set_tracer_provider(provider.clone());
    provider.tracer("tracing-otel-subscriber")
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
