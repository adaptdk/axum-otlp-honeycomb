//! Error types for the Axum OTLP Honeycomb integration.
//!

use thiserror::Error;
#[derive(Error, Debug)]
pub enum AxumOtlpHoneycombError {
    #[error("Cannot append path to endpoint URL: {0}")]
    CannotAppendPath(#[from] url::ParseError),

    #[error("Cannot build exporter: {0}")]
    OtlpTraceProviderInitError(#[from] opentelemetry_otlp::ExporterBuildError),

    #[error("Missing HoneycombParams ingest key")]
    MissingIngestKey,

    #[error("Missing HoneycombParams endpoint")]
    MissingEndpoint,

    #[error("Missing HoneycombParams service name")]
    MissingServiceName,

    #[error("Missing service version")]
    MissingServiceVersion,
}
