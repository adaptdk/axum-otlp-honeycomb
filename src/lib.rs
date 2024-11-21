//! Crate for connecting tracing in Axum via the Opengtelemetry-otlp
//! protocol to Honeycomb.

use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::{SpanExporter, WithHttpConfig};
use opentelemetry_sdk::{
    self as sdk,
    trace::{Config, Sampler, Tracer},
};
use tracing_core::Subscriber;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::registry::LookupSpan;
mod axum_layer;
pub use axum_layer::{opentelemetry_tracing_layer, opentelemetry_tracing_layer_without_parent};

/// Creates a layer that can be added to a `tracing_subscriber`like this
///
/// ```
/// let sample_rate = 0.01;  // 1%
/// tracing_subscriber::Registry::default()
///    .with(init_otlp_layer(sample_rate).with_filter(LevelFilter::INFO))
///    .init();
/// ```
///
/// The `sample_rate` is the fraction of traces that should be sent to Honeycomb.
/// 1.0 is all traces.
///
/// Also sets a `text_map_propagator` to enable propagation
/// of context between services.
///
/// Expects the following environment variables:
/// *  `HONEYCOMB_API_KEY` contains
///     the API key for the Honeycomb environment that traces should be sent to
/// *  `OTEL_EXPORTER_OTLP_ENDPOINT` contains the endpoint for Honeycomb -
///     eg `https://api.eu1.honeycomb.io/`
/// *  `OTEL_SERVICE_NAME` contains the service name - eg `clap::crate_name!()`.
pub fn init_otlp_layer<S>(sample_rate: f64) -> Option<OpenTelemetryLayer<S, Tracer>>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );

    #[allow(clippy::expect_used)] // Should panic as we cannot continue
    let headers = vec![(
        "x-honeycomb-team".to_string(),
        std::env::var("HONEYCOMB_API_KEY").expect("Env var HONEYCOMB_API_KEY missing"),
    )];
    let exporter = SpanExporter::builder()
        .with_http()
        .with_headers(headers.into_iter().collect())
        .build();

    if exporter.is_err() {
        None
    } else {
        let provider = sdk::trace::TracerProvider::builder()
            .with_batch_exporter(exporter.unwrap(), opentelemetry_sdk::runtime::Tokio)
            .with_config(
                Config::default().with_sampler(Sampler::ParentBased(Box::new(
                    Sampler::TraceIdRatioBased(sample_rate),
                ))),
            )
            .build();
        let tracer = provider.tracer("axum-otlp-honeycomb");

        let l = tracing_opentelemetry::layer()
            .with_error_records_to_exceptions(true)
            .with_tracer(tracer);
        Some(l)
    }
}
