//! Crate for connecting tracing in Axum via the Opengtelemetry-otlp
//! protocol to Honeycomb.

mod axum_layer;
mod error;
mod event_logger;
mod providers;

pub use axum_layer::{opentelemetry_tracing_layer, opentelemetry_tracing_layer_without_parent};
pub use event_logger::AxumOtelEventLogger;
pub use providers::{
    HoneycombParamsBuilder, OtelGuard, init_otlp_log_provider, init_otlp_metrics_provider,
    init_otlp_trace_provider,
};
