//! Crate for connecting tracing in Axum via the Opengtelemetry-otlp
//! protocol to Honeycomb.

use event_logger::AxumOtelEventLogger;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::{LogExporter, SpanExporter};
use opentelemetry_sdk::{
    self as sdk,
    logs::{SdkLogger, SdkLoggerProvider},
    trace::{Sampler, Tracer},
};
use tracing_core::Subscriber;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::registry::LookupSpan;
mod axum_layer;
mod event_logger;
pub use axum_layer::{opentelemetry_tracing_layer, opentelemetry_tracing_layer_without_parent};

/// Creates a tracing layer that can be added to a `tracing_subscriber`like this
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
///    the API key for the Honeycomb environment that traces should be sent to
/// *  `OTEL_EXPORTER_OTLP_ENDPOINT` contains the endpoint for Honeycomb -
///    eg `https://api.eu1.honeycomb.io/`
/// *  `OTEL_SERVICE_NAME` contains the service name - eg `clap::crate_name!()`.
pub fn init_otlp_layer<S>(sample_rate: f64) -> Option<OpenTelemetryLayer<S, Tracer>>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );

    if let Ok(exporter) = SpanExporter::builder().with_http().build() {
        let provider = sdk::trace::SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                sample_rate,
            ))))
            .build();
        let tracer = provider.tracer("axum-otlp-honeycomb");
        let layer = tracing_opentelemetry::layer()
            .with_level(true)
            .with_tracer(tracer);
        Some(layer)
    } else {
        None
    }
}

/// Creates an event logging layer that can be added to a `tracing_subscriber`like this
///
/// ```
/// tracing_subscriber::Registry::default()
///    .with(init_otlp_log_ layer().with_filter(LevelFilter::INFO))
///    .init();
/// ```
///
/// This layer sends events (with level greater than or equal to INFO) onwards
/// to Honeycomb as Logs.
///
/// IMPORTANT: The body of the event is defined by the `log` and `tracing` crates
/// to be in the field `message`.  In `opentelemetry` this is moved to the `body`
/// field. Any field in the event with the name `body` will overwrite the event message.
///
/// Expects the same environment variables as `init_otlp_log_layer()`
pub fn init_otlp_log_layer() -> AxumOtelEventLogger<SdkLoggerProvider, SdkLogger> {
    let exporter = LogExporter::builder().with_http().build().unwrap();
    let provider = sdk::logs::SdkLoggerProvider::builder()
        .with_batch_exporter(exporter)
        .build();
    AxumOtelEventLogger::new(&provider)
}
