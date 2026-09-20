#![allow(dead_code)]
use crate::config::Config;
use tracing::info;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use base64::Engine;
use opentelemetry::{global, trace::TracerProvider};
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
use std::io::Error;
use tonic::metadata::*;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};

pub struct OtelGuard {
    meter_provider: Option<SdkMeterProvider>,
}
impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Some(mp) = self.meter_provider.take() {
            if let Err(err) = mp.shutdown() {
                eprintln!("meter_provider shutdown error: {err:?}");
            }
        }
        global::shutdown_tracer_provider();
    }
}

pub fn init_telemetry(
    config: &Config,
    resource: Resource,
    filter: EnvFilter,
) -> anyhow::Result<OtelGuard> {
    // Пытаемся поднять OTLP — но если не получилось, просто логируем и идём дальше.
    let (meter_provider, tracer) = match (
        init_meter_provider(config, resource.clone()),
        init_tracer(config, resource.clone()),
    ) {
        (Ok(mp), Ok(tr)) => (Some(mp), Some(tr)),
        (mp_res, tr_res) => {
            if let Err(e) = &mp_res {
                eprintln!("metrics init failed: {e:#}");
            }
            if let Err(e) = &tr_res {
                eprintln!("tracer init failed: {e:#}");
            }
            (None, None)
        }
    };

    let fmt_layer = tracing_subscriber::fmt::layer().json();
    let registry = tracing_subscriber::registry().with(filter).with(fmt_layer);

    // Ставим subscriber в зависимости от того, что удалось создать.
    match (meter_provider.clone(), tracer) {
        (Some(mp), Some(tr)) => {
            let otel_layer = OpenTelemetryLayer::new(tr);
            let result = registry
                .with(otel_layer)
                .with(MetricsLayer::new(mp))
                .try_init();
            if let Err(e) = result {
                eprintln!("subscriber already set: {e}");
            }
        }
        _ => {
            let result = registry.try_init();
            if let Err(e) = result {
                eprintln!("subscriber already set: {e}");
            }
            eprintln!("OpenTelemetry disabled — running with fmt subscriber only");
        }
    }

    Ok(OtelGuard { meter_provider })
}

fn init_meter_provider(config: &Config, resource: Resource) -> anyhow::Result<SdkMeterProvider> {
    let endpoint = config.otel_endpoint.as_str();
    info!("init_meter_provider endpoint {}", endpoint);

    let metadata = otl_metadata(config)?;

    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint)
        .with_protocol(opentelemetry_otlp::Protocol::Grpc)
        .with_metadata(metadata)
        .build_metrics_exporter(
            Box::new(DefaultAggregationSelector::new()),
            Box::new(DefaultTemporalitySelector::new()),
        )?;
    let reader = PeriodicReader::builder(exporter, runtime::Tokio)
        .with_interval(std::time::Duration::from_secs(30))
        .build();

    let view_duration = |instrument: &Instrument| -> Option<Stream> {
        if instrument.name == "http_request_duration_seconds" {
            Some(
                Stream::new().aggregation(Aggregation::ExplicitBucketHistogram {
                    boundaries: vec![
                        0.0005, 0.001, 0.002, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5,
                        5.0, 10.0,
                    ],
                    record_min_max: true,
                }),
            )
        } else {
            None
        }
    };

    let meter_provider = MeterProviderBuilder::default()
        .with_resource(resource)
        .with_reader(reader)
        .with_view(view_duration)
        .build();
    global::set_meter_provider(meter_provider.clone());
    Ok(meter_provider)
}

fn init_tracer(config: &Config, resource: Resource) -> anyhow::Result<Tracer> {
    let endpoint = config.otel_endpoint.as_str();
    info!("init_tracer endpoint {}", endpoint);

    let metadata = otl_metadata(config)?;

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                    1.0,
                ))))
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource),
        )
        .with_batch_config(BatchConfig::default())
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint)
                .with_metadata(metadata),
        )
        .install_batch(runtime::Tokio)?;
    global::set_tracer_provider(provider.clone());
    Ok(provider.tracer("tracing-otel-subscriber"))
}

fn otl_metadata(config: &Config) -> anyhow::Result<MetadataMap, Error> {
    let auth_string = format!("{}:{}", config.otel_user, config.otel_password);
    let base64_token = base64::engine::general_purpose::STANDARD.encode(auth_string);
    let auth_header_value = format!("basic {}", base64_token);

    let mut map = MetadataMap::with_capacity(3);
    map.insert("authorization", auth_header_value.parse().unwrap());
    map.insert("organization", "default".parse().unwrap());
    map.insert("stream-name", "default".parse().unwrap());
    Ok(map)
}
