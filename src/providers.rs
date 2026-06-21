//! Providers and guard type for tracing, logging and metrics
//!
//! ...
use crate::error::AxumOtlpHoneycombError;
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::{
    LogExporter, MetricExporter, SpanExporter, WithExportConfig, WithHttpConfig,
};
use opentelemetry_sdk::{
    self as sdk, Resource,
    logs::SdkLoggerProvider,
    metrics::{PeriodicReader, SdkMeterProvider},
    trace::{Sampler, SdkTracerProvider},
};
use opentelemetry_semantic_conventions::resource::SERVICE_VERSION;
use std::collections::HashMap;
use url::Url;

/// Creates a tracing layer that can be added to a `tracing_subscriber`like this
///
/// ```
/// let sample_rate = 0.01;  // 1%
/// tracing_subscriber::Registry::default()
///    .with(init_otlp_layer(sample_rate, &HoneycombParams)
///    .with_filter(LevelFilter::INFO))
///    .init();
/// ```
///
/// The `sample_rate` is the fraction of traces that should be sent to Honeycomb.
/// 1.0 is all traces.
///
/// Also sets a `text_map_propagator` to enable propagation
/// of context between services.
///
pub fn init_otlp_trace_provider(
    sample_rate: f64,
    honeycomb_params: &HoneycombParams,
) -> Result<SdkTracerProvider, AxumOtlpHoneycombError> {
    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );

    let exporter = SpanExporter::builder()
        .with_http()
        .with_endpoint(honeycomb_params.endpoint.join("./v1/traces")?)
        .with_headers(honeycomb_headers(honeycomb_params))
        .build()?;

    let provider = sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
            sample_rate,
        ))))
        .with_resource(honeycomb_ressource(honeycomb_params))
        .build();
    Ok(provider)
}

/// Creates an event logging layer that can be added to a `tracing_subscriber`like this
///
/// ```ignore
/// tracing_subscriber::Registry::default()
///    .with(init_otlp_log_provider(&HoneycombParams)
///    .with_filter(LevelFilter::INFO))
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
pub fn init_otlp_log_provider(
    honeycomb_params: &HoneycombParams,
) -> Result<SdkLoggerProvider, AxumOtlpHoneycombError> {
    let exporter = LogExporter::builder()
        .with_http()
        .with_endpoint(honeycomb_params.endpoint.join("./v1/logs")?)
        .with_headers(honeycomb_headers(honeycomb_params))
        .build()?;
    let provider = sdk::logs::SdkLoggerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(honeycomb_ressource(honeycomb_params))
        .build();
    Ok(provider)
}

/// Creates a metrics logging layer that exports metrics to Honeycomb.
///
/// The metrics are counts and distribution of requests to all end-points
/// and the number of active requests.
///
/// ```ignore
/// let metrics_interval = std::time::Duration::from_secs(60);
/// tracing_subscriber::Registry::default()
///     .with(init_otlp_metrics_layer(metrics_interval, &honeycomb_params))
///     .build()
/// ```
/// The metrics are exported every `metrics_interval`.
pub fn init_otlp_metrics_provider(
    metrics_interval: std::time::Duration,
    honeycomb_params: &HoneycombParams,
) -> Result<SdkMeterProvider, AxumOtlpHoneycombError> {
    let exporter = MetricExporter::builder()
        .with_http()
        .with_endpoint(honeycomb_params.endpoint.join("./v1/metrics")?)
        .with_headers(honeycomb_headers(honeycomb_params))
        .build()?;
    println!("Exporter: {exporter:#?}");
    let reader = PeriodicReader::builder(exporter)
        .with_interval(metrics_interval)
        .build();
    let provider = SdkMeterProvider::builder()
        .with_resource(honeycomb_ressource(honeycomb_params))
        .with_reader(reader)
        .build();

    global::set_meter_provider(provider.clone());
    Ok(provider)
}

/// A guard that holds the OTLP providers and shuts them down when dropped.
///
/// Drop on this should be called at the end of the application lifecycle,
/// and for example on Upsun in the ctrl-c handler.
pub struct OtelGuard {
    tracer_provider: SdkTracerProvider,
    logger_provider: SdkLoggerProvider,
    meter_provider: SdkMeterProvider,
}

impl OtelGuard {
    /// Creates a new `OtelGuard` with the given OTLP providers.
    pub fn new(
        tracer_provider: SdkTracerProvider,
        logger_provider: SdkLoggerProvider,
        meter_provider: SdkMeterProvider,
    ) -> Self {
        Self {
            tracer_provider,
            logger_provider,
            meter_provider,
        }
    }
}

impl Drop for OtelGuard {
    /// Called to shut down the OTLP providers when the `OtelGuard` is dropped.
    fn drop(&mut self) {
        if let Err(err) = self.tracer_provider.shutdown() {
            eprintln!("Tracer shutdown error: {err:?}");
        }
        if let Err(err) = self.meter_provider.shutdown() {
            eprintln!("Meter shutdown error: {err:?}");
        }
        if let Err(err) = self.logger_provider.shutdown() {
            eprintln!("Logger shutdown error: {err:?}");
        }
    }
}

fn honeycomb_ressource(honeycomb_params: &HoneycombParams) -> Resource {
    opentelemetry_sdk::Resource::builder()
        .with_service_name(honeycomb_params.service_name.clone())
        .with_attributes(vec![KeyValue::new(
            SERVICE_VERSION,
            honeycomb_params.service_version.clone(),
        )])
        .build()
}

fn honeycomb_headers(honeycomb_params: &HoneycombParams) -> HashMap<String, String> {
    let mut headers: HashMap<String, String> = HashMap::new();
    headers.insert(
        "x-honeycomb-team".to_string(),
        honeycomb_params.ingest_key.clone(),
    );
    headers.insert(
        "x-honeycomb-dataset".to_string(),
        honeycomb_params.service_name.clone(),
    );
    headers
}

/// Struct with the params needed to configure Honeycomb tracing.
pub struct HoneycombParams {
    /// Honeycomb ingest key.
    ingest_key: String,
    /// Honeycomb endpoint URL.
    endpoint: Url,
    /// Service-name or Honeycomb dataset name.
    service_name: String,
    /// Version id so Honeycomb can find deployments.
    service_version: String,
}

/// Builder for creating a [`HoneycombParams`] struct.
pub struct HoneycombParamsBuilder {
    ingest_key: Option<String>,
    endpoint: Option<Url>,
    service_name: Option<String>,
    service_version: Option<String>,
}

impl HoneycombParamsBuilder {
    /// Creates a new HoneycombParamsBuilder with the given ingest key.
    ///
    /// The endpoint is set to the default EU Honeycomb endpoint
    /// `https://api.eu1.honeycomb.io/`.
    pub fn new(ingest_key: String) -> Self {
        Self {
            ingest_key: Some(ingest_key),
            endpoint: Some(Url::parse("https://api.eu1.honeycomb.io/").unwrap()),
            service_name: None,
            service_version: None,
        }
    }

    /// Sets the endpoint for Honeycomb if different from the default.
    ///
    /// The endpoint must be a valid URL.
    ///
    /// The default `/v1/traces`, `/v1/logs` and `/v1/metrics` paths are appended automatically.
    pub fn with_endpoint(mut self, endpoint: Url) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    /// Sets the service name or dataset name for Honeycomb.
    pub fn with_service_name(mut self, service_name: String) -> Self {
        self.service_name = Some(service_name);
        self
    }

    /// Sets the service version for Honeycomb.
    pub fn with_service_version(mut self, service_version: String) -> Self {
        self.service_version = Some(service_version);
        self
    }

    /// Builds the HoneycombParams from the builder.
    ///
    /// Returns `None` if any of the required fields are not set.
    pub fn build(self) -> Result<HoneycombParams, AxumOtlpHoneycombError> {
        let ingest_key = self
            .ingest_key
            .ok_or(AxumOtlpHoneycombError::MissingIngestKey)?;
        let endpoint = self
            .endpoint
            .ok_or(AxumOtlpHoneycombError::MissingEndpoint)?;
        let service_name = self
            .service_name
            .ok_or(AxumOtlpHoneycombError::MissingServiceName)?;
        let service_version = self
            .service_version
            .ok_or(AxumOtlpHoneycombError::MissingServiceVersion)?;
        Ok(HoneycombParams {
            ingest_key,
            endpoint,
            service_name,
            service_version,
        })
    }
}
