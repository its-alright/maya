#![allow(dead_code)]
use crate::config::Config;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

use base64::Engine;
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_sdk::{
    Resource,
    logs::SdkLoggerProvider,
    metrics::{Aggregation, Instrument, PeriodicReader, SdkMeterProvider, Stream},
    trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
};
use tonic::metadata::*;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};

use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{
    LogExporter, MetricExporter, SpanExporter, WithExportConfig, WithTonicConfig,
};

pub struct OtelGuard {
    meter_provider: Option<SdkMeterProvider>,
    tracer_provider: Option<SdkTracerProvider>,
    logger_provider: Option<SdkLoggerProvider>,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Some(mp) = self.meter_provider.take() {
            let _ = mp.shutdown();
        }
        if let Some(tp) = self.tracer_provider.take() {
            let _ = tp.shutdown();
        }
        if let Some(lp) = self.logger_provider.take() {
            let _ = lp.shutdown();
        }
    }
}

pub fn init_telemetry(
    config: &Config,
    resource: Resource,
    filter: EnvFilter,
) -> anyhow::Result<OtelGuard> {
    let metadata = otl_metadata(config);

    // 1. Создаём экспортеры через новые Builder-паттерны
    let log_exporter = LogExporter::builder()
        .with_tonic()
        .with_endpoint(config.otel_endpoint.clone())
        .with_metadata(metadata.clone())
        .build()?;

    let span_exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(config.otel_endpoint.clone())
        .with_metadata(metadata.clone())
        .build()?;

    let metric_exporter = MetricExporter::builder()
        .with_tonic()
        .with_endpoint(config.otel_endpoint.clone())
        .with_metadata(metadata.clone())
        .build()?;

    // 2. Создаём провайдеры с batch-экспортерами
    let logger_provider = SdkLoggerProvider::builder()
        .with_resource(resource.clone())
        .with_batch_exporter(log_exporter)
        .build();

    let tracer_provider = SdkTracerProvider::builder()
        .with_resource(resource.clone())
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
            1.0,
        ))))
        .with_id_generator(RandomIdGenerator::default())
        .with_batch_exporter(span_exporter)
        .build();

    let meter_provider = init_meter_provider(metric_exporter, resource.clone())?;

    // let meter_provider = SdkMeterProvider::builder()
    //     .with_resource(resource.clone())
    //     .with_periodic_exporter(metric_exporter)
    //     .build();

    // 3. Устанавливаем глобальные провайдеры
    global::set_tracer_provider(tracer_provider.clone());
    global::set_meter_provider(meter_provider.clone());

    // 4. Настраиваем tracing layers
    let otel_trace_layer = OpenTelemetryLayer::new(tracer_provider.tracer("tracing-otel"));
    let otel_log_layer = OpenTelemetryTracingBridge::new(&logger_provider);
    let otel_metrics_layer = MetricsLayer::new(meter_provider.clone());

    // 5. Фильтр для OTLP слоёв, чтобы избежать рекурсии
    let otel_filter = EnvFilter::new("info")
        .add_directive("opentelemetry=off".parse()?)
        .add_directive("hyper=off".parse()?)
        .add_directive("tonic=off".parse()?);

    let fmt_layer = tracing_subscriber::fmt::layer().json();

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(otel_trace_layer)
        .with(otel_log_layer.with_filter(otel_filter))
        .with(otel_metrics_layer)
        .try_init()?;

    Ok(OtelGuard {
        meter_provider: Some(meter_provider),
        tracer_provider: Some(tracer_provider),
        logger_provider: Some(logger_provider),
    })
}

fn init_meter_provider(
    exporter: MetricExporter,
    resource: Resource,
) -> anyhow::Result<SdkMeterProvider> {
    // PeriodicReader с интервалом 30 секунд — аналог старого PeriodicReader::builder
    let reader = PeriodicReader::builder(exporter)
        .with_interval(std::time::Duration::from_secs(15))
        .build();

    // Кастомные границы для http_request_duration_seconds
    let view_duration = |instrument: &Instrument| -> Option<Stream> {
        if instrument.name() == "http_request_duration_seconds" {
            Some(
                Stream::builder()
                    .with_aggregation(Aggregation::ExplicitBucketHistogram {
                        boundaries: vec![
                            0.0005, 0.001, 0.002, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0,
                            2.5, 5.0, 10.0,
                        ],
                        record_min_max: true,
                    })
                    .build()
                    .unwrap(), // Stream::new().aggregation(Aggregation::ExplicitBucketHistogram {
                               //     boundaries: vec![
                               //         0.0005, 0.001, 0.002, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5,
                               //         5.0, 10.0,
                               //     ],
                               //     record_min_max: true,
                               // }),
            )
        } else {
            None
        }
    };

    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(reader)
        .with_view(view_duration)
        .build();

    Ok(meter_provider)
}

// fn init_meter_provider2(config: &Config, resource: Resource) -> anyhow::Result<SdkMeterProvider> {
//     let endpoint = config.otel_endpoint.as_str();
//     let metadata = otl_metadata(config);
//
//     let exporter = opentelemetry_otlp::new_exporter()
//         .tonic()
//         .with_endpoint(endpoint)
//         .with_protocol(opentelemetry_otlp::Protocol::Grpc)
//         .with_metadata(metadata)
//         .build_metrics_exporter(
//             Box::new(DefaultAggregationSelector::new()),
//             Box::new(DefaultTemporalitySelector::new()),
//         )?;
//     let reader = PeriodicReader::builder(exporter, runtime::Tokio)
//         .with_interval(std::time::Duration::from_secs(30))
//         .build();
//
//     let view_duration = |instrument: &Instrument| -> Option<Stream> {
//         if instrument.name == "http_request_duration_seconds" {
//             Some(
//                 Stream::new().aggregation(Aggregation::ExplicitBucketHistogram {
//                     boundaries: vec![
//                         0.0005, 0.001, 0.002, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5,
//                         5.0, 10.0,
//                     ],
//                     record_min_max: true,
//                 }),
//             )
//         } else {
//             None
//         }
//     };
//
//     let meter_provider = MeterProviderBuilder::default()
//         .with_resource(resource)
//         .with_reader(reader)
//         .with_view(view_duration)
//         .build();
//     global::set_meter_provider(meter_provider.clone());
//     Ok(meter_provider)
// }
//
// fn init_tracer(config: &Config, resource: Resource) -> anyhow::Result<Tracer> {
//     let endpoint = config.otel_endpoint.as_str();
//     info!("init_tracer endpoint {}", endpoint);
//
//     let metadata = otl_metadata(config)?;
//
//     let provider = opentelemetry_otlp::new_pipeline()
//         .tracing()
//         .with_trace_config(
//             opentelemetry_sdk::trace::Config::default()
//                 .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
//                     1.0,
//                 ))))
//                 .with_id_generator(RandomIdGenerator::default())
//                 .with_resource(resource),
//         )
//         .with_batch_config(BatchConfig::default())
//         .with_exporter(
//             opentelemetry_otlp::new_exporter()
//                 .tonic()
//                 .with_endpoint(endpoint)
//                 .with_metadata(metadata),
//         )
//         .install_batch(runtime::Tokio)?;
//     global::set_tracer_provider(provider.clone());
//     Ok(provider.tracer("tracing-otel-subscriber"))
// }

fn otl_metadata(config: &Config) -> MetadataMap {
    let auth_string = format!("{}:{}", config.otel_user, config.otel_password);
    let base64_token = base64::engine::general_purpose::STANDARD.encode(auth_string);
    let auth_header_value = format!("basic {}", base64_token);

    let mut map = MetadataMap::with_capacity(3);
    map.insert("authorization", auth_header_value.parse().unwrap());
    map.insert("organization", "default".parse().unwrap());
    map.insert("stream-name", "default".parse().unwrap());

    map
}
