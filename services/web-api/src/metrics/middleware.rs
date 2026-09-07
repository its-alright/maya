use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use opentelemetry::metrics::Histogram;
use opentelemetry::{KeyValue, global, metrics::Meter};
use std::sync::Arc;
use std::time::Instant;

pub struct MetricsMiddleware {
    request_counter: opentelemetry::metrics::Counter<u64>,
    error_counter: opentelemetry::metrics::Counter<u64>,
    duration_histogram: opentelemetry::metrics::Histogram<f64>,
}

impl MetricsMiddleware {
    pub fn new() -> Self {
        let meter = global::meter("web-api");

        let request_counter = meter
            .u64_counter("http_requests_total")
            .with_description("Total number of HTTP requests")
            .init();

        let error_counter = meter
            .u64_counter("http_errors_total")
            .with_description("Total number of HTTP errors")
            .init();

        let duration_histogram = meter
            .f64_histogram("http_request_duration_seconds")
            .with_description("HTTP request duration in seconds")
            .init();

        Self {
            request_counter,
            error_counter,
            duration_histogram,
        }
    }

    pub async fn layer(&self, request: Request, next: Next) -> Response {
        let start = Instant::now();
        let method = request.method().to_string();
        let path = request.uri().path().to_string();

        let handler_name = tracing::Span::current()
            .metadata()
            .unwrap()
            .name()
            .to_string();

        // Обрабатываем запрос
        let response = next.run(request).await;

        let duration = start.elapsed().as_secs_f64();
        let status = response.status();
        let status_code = status.as_u16();

        // Атрибуты для метрик
        let attributes = vec![
            KeyValue::new("method", method.clone()),
            KeyValue::new("path", path.clone()),
            KeyValue::new("handler", handler_name),
            KeyValue::new("status_code", status_code.to_string()),
            KeyValue::new(
                "status_type",
                if status_code >= 500 {
                    "error"
                } else {
                    "success"
                },
            ),
        ];

        // 1. Счетчик запросов (RPS)
        self.request_counter.add(1, &attributes);

        // 2. Счетчик ошибок (5xx)
        if status.is_server_error() {
            self.error_counter.add(1, &attributes);
        }

        // 3. Гистограмма длительности (p50, p90, p99)
        self.duration_histogram.record(duration, &attributes);

        response
    }
}
