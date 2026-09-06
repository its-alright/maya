use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use tower_http::request_id::RequestId;
use tracing::{info, error, warn, span, Level};
use tracing::field::display;

pub async fn request_logging_layer(request: Request, next: Next) -> Response {
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .map(|id| id.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let method = request.method().clone();
    let uri = request.uri().clone();
    let version = request.version();
    let headers = request.headers().clone();

    let span = span!(
        Level::INFO,
        "http_request",
        request_id = %request_id,
        method = %method,
        path = %uri.path(),
        version = ?version,
    );

    let _enter = span.enter();

    info!(
        request_id = %request_id,
        method = %method,
        path = %uri.path(),
        headers = ?headers,
        "Incoming request"
    );

    let start = std::time::Instant::now();
    let response = next.run(request).await;
    let duration = start.elapsed();

    let status = response.status();

    if status.is_success() {
        info!(
            request_id = %request_id,
            status = %status,
            duration_ms = duration.as_millis(),
            "Request completed successfully"
        );
    } else if status.is_client_error() {
        warn!(
            request_id = %request_id,
            status = %status,
            duration_ms = duration.as_millis(),
            "Client error"
        );
    } else {
        error!(
            request_id = %request_id,
            status = %status,
            duration_ms = duration.as_millis(),
            "Server error"
        );
    }

    response
}