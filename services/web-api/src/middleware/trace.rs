use axum::{extract::Request, middleware::Next, response::Response};
use opentelemetry::trace::{TraceContextExt, TraceState};
use tracing::{info, info_span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub async fn extract_trace_context(req: Request, next: Next) -> Response {
    // Извлекаем заголовок traceparent
    if let Some(traceparent) = req.headers().get("traceparent") {
        if let Ok(tp_str) = traceparent.to_str() {
            info!("Web API traceparent:{}", tp_str);
            // Парсим W3C Trace Context: 00-{trace_id}-{span_id}-{flags}
            let parts: Vec<&str> = tp_str.split('-').collect();
            if parts.len() >= 4 {
                let trace_id = parts[1];
                let span_id = parts[2];
                let flags = parts.get(3).unwrap_or(&"01");

                let span_context = opentelemetry::trace::SpanContext::new(
                    opentelemetry::trace::TraceId::from_hex(trace_id).unwrap(),
                    opentelemetry::trace::SpanId::from_hex(span_id).unwrap(),
                    opentelemetry::trace::TraceFlags::new(
                        u8::from_str_radix(flags, 16).unwrap_or(1),
                    ),
                    false,
                    TraceState::NONE,
                );

                // Создаем Context и устанавливаем как родительский
                let parent_context =
                    opentelemetry::Context::new().with_remote_span_context(span_context);

                // Создаем span с родительским контекстом
                let span = info_span!("web_api_request");
                span.set_parent(parent_context);
                let _guard = span.enter();

                return next.run(req).await;
            }
        }
    }

    // Если заголовка нет — создаем новый span
    next.run(req).await
}
