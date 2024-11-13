//! Crate for connecting tracing in Axum via the Opengtelemetry-otlp
//! protocol to Honeycomb.

use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use opentelemetry::trace::TracerProvider;
use std::env;
use tracing_core::Subscriber;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::registry::LookupSpan;

/// Creates a layer that can be added to a `tracing_subscriber`like this
///
/// ```
///     tracing_subscriber::Registry::default()
///    .with(init_otlp_layer().with_filter(LevelFilter::INFO))
///    .init();
/// ```
///
/// Also sets a `text_map_propagator` to enable propagation
/// of context between services.
///
/// Expects the following environment variables:
/// *  `HONEYCOMB_API_KEY` contains
///     the API key for the Honeycomb environment that traces should be sent to
/// *  `OTEL_EXPORTER_OTLP_ENDPOINT` contains the endpoint for Honeycomb -
///     default is `https://api.eu1.honeycomb.io/`
/// *  `OTEL_SERVICE_NAME` contains the service name. Defaults to the package
///     name from Cargo.toml
pub fn init_otlp_layer<S>() -> Option<OpenTelemetryLayer<S, opentelemetry_sdk::trace::Tracer>>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    env::set_var(
        "OTEL_EXPORTER_OTLP_ENDPOINT",
        env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .unwrap_or("https://api.eu1.honeycomb.io/".to_string()),
    );
    env::set_var(
        "OTEL_SERVICE_NAME",
        env::var("OTEL_SERVICE_NAME")
            .unwrap_or(env::var("CARGO_PKG_NAME").unwrap_or("unknown".to_string())),
    );

    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );

    create_otlp_tracer().map(|t| {
        tracing_opentelemetry::layer()
            .with_error_records_to_exceptions(true)
            .with_tracer(t)
    })
}

fn create_otlp_tracer() -> Option<opentelemetry_sdk::trace::Tracer> {
    #[allow(clippy::expect_used)] // Should panic as we cannot continue
    let headers = vec![(
        "x-honeycomb-team".to_string(),
        std::env::var("HONEYCOMB_API_KEY").expect("Env var HONEYCOMB_API_KEY missing"),
    )];
    let exporter = opentelemetry_otlp::new_exporter()
        .http()
        .with_headers(headers.into_iter().collect());
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter);
    Some(
        tracer
            .install_batch(opentelemetry_sdk::runtime::Tokio)
            .unwrap()
            .tracer(""),
    )
}

/// Creates a new tracing span
///
/// Just axum-tracing-opentelemetry's struct for now
#[must_use]
pub fn opentelemetry_tracing_layer() -> OtelAxumLayer {
    OtelAxumLayer::default()
}

/// Injects the context into the response
///
/// Just axum-tracing-opentelemetry's struct for now
#[must_use]
pub fn response_with_trace_layer() -> OtelInResponseLayer {
    OtelInResponseLayer {}
}
