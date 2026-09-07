use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use opentelemetry::{global, KeyValue};
use std::time::Instant;
use tracing::info;

pub struct MetricsMiddleware {
    request_counter: opentelemetry::metrics::Counter<u64>,
    error_counter: opentelemetry::metrics::Counter<u64>,
    duration_histogram: opentelemetry::metrics::Histogram<f64>,
}

impl MetricsMiddleware {
    pub fn new() -> Self {
        let meter = global::meter("web-api");

        Self {
            request_counter: meter
                .u64_counter("http_requests_total")
                .with_description("Total HTTP requests")
                .init(),
            error_counter: meter
                .u64_counter("http_errors_total")
                .with_description("Total HTTP errors")
                .init(),
            duration_histogram: meter
                .f64_histogram("http_request_duration_seconds")
                .with_description("HTTP request duration")
                .init(),
        }
    }

    pub async fn layer(&self, request: Request, next: Next) -> Response {
        let start = Instant::now();

        let method = request.method().to_string();
        let path = request.uri().path().to_string();

        info!("method:{} path:{}", method, path);

        let response = next.run(request).await;

        let duration = start.elapsed().as_secs_f64();
        let status_code = response.status().as_u16();

        let attributes = vec![
            KeyValue::new("method", method),
            KeyValue::new("path", path),
            KeyValue::new("status_code", status_code.to_string()),
            KeyValue::new("status_type", if status_code >= 500 { "error" } else { "success" }),
        ];

        self.request_counter.add(1, &attributes);

        if status_code >= 500 {
            self.error_counter.add(1, &attributes);
        }

        self.duration_histogram.record(duration, &attributes);

        response
    }
}