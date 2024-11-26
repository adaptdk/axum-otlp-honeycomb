//! Logging of events

use opentelemetry::{
    logs::{AnyValue, LogRecord, Logger, LoggerProvider, Severity},
    InstrumentationScope, Key,
};
use std::borrow::Cow;
use tracing::Level;
use tracing_subscriber::{registry::LookupSpan, Layer};
const INSTRUMENTATION_LIBRARY_NAME: &str = "axum_otel_honeycomb";

pub struct AxumOtelEventLogger<P, L>
where
    P: LoggerProvider<Logger = L> + Send + Sync,
    L: Logger + Send + Sync,
{
    logger: L,
    _phantom: std::marker::PhantomData<P>, // P is not used.
}

impl<P, L> AxumOtelEventLogger<P, L>
where
    P: LoggerProvider<Logger = L> + Send + Sync,
    L: Logger + Send + Sync,
{
    pub fn new(provider: &P) -> Self {
        let scope = InstrumentationScope::builder(INSTRUMENTATION_LIBRARY_NAME)
            .with_version(Cow::Borrowed(env!("CARGO_PKG_VERSION")))
            .build();

        AxumOtelEventLogger {
            logger: provider.logger_with_scope(scope),
            _phantom: Default::default(),
        }
    }
}

/// All data and metadata from the span.
#[derive(Debug)]
struct ExtensionValues {
    span_str: String,
    location: String,
}

impl<S, P, L> Layer<S> for AxumOtelEventLogger<P, L>
where
    S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
    P: LoggerProvider<Logger = L> + Send + Sync + 'static,
    L: Logger + Send + Sync + 'static,
{
    fn on_new_span(
        &self,
        attrs: &tracing_core::span::Attributes<'_>,
        id: &tracing_core::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Some(span) = ctx.span(id) {
            let mut span_str = String::with_capacity(256);
            let location = format!(
                "{file}:{line}",
                file = attrs.metadata().file().unwrap_or("UNKNOWN"),
                line = attrs.metadata().line().unwrap_or_default(),
            );
            span_str.push_str(&format!(
                "Name: '{name}', {{ module: '{module}', location: '{location}'",
                name = attrs.metadata().name(),
                module = attrs.metadata().module_path().unwrap_or_default(),
                location = location,
            ));

            let mut visitor = SpanVisitor::new(&mut span_str);
            attrs.values().record(&mut visitor);
            span_str.push_str(" }");
            let extension = ExtensionValues { span_str, location };
            span.extensions_mut().insert(extension);
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let meta = event.metadata();

        let mut log_record = self.logger.create_log_record();

        // TODO: Fix heap allocation
        log_record.set_target(meta.target().to_string());
        log_record.set_event_name(meta.name());
        log_record.set_severity_number(severity_of_level(meta.level()));
        log_record.set_severity_text(meta.level().as_str());
        log_record.add_attribute(
            "location",
            format!(
                "{}:{}",
                meta.file().unwrap_or("UNKNOWN"),
                meta.line().unwrap_or_default()
            ),
        );
        let mut visitor = EventVisitor::new(&mut log_record);
        // Visit fields.
        event.record(&mut visitor);
        // Log spans.
        if let Some(scope) = ctx.event_scope(event) {
            for (i, span) in scope.from_root().enumerate() {
                let ext = span.extensions();
                if let Some(span_data) = ext.get::<ExtensionValues>() {
                    log_record.add_attribute(format!("span.{i}"), span_data.span_str.clone());
                    log_record
                        .add_attribute(format!("span.{i}.location"), span_data.location.clone());
                }
                log_record.add_attribute(format!("span.{i}.name"), span.name());
            }
        }
        //emit record
        self.logger.emit(log_record);
    }
}

const fn severity_of_level(level: &Level) -> Severity {
    match *level {
        Level::TRACE => Severity::Trace,
        Level::DEBUG => Severity::Debug,
        Level::INFO => Severity::Info,
        Level::WARN => Severity::Warn,
        Level::ERROR => Severity::Error,
    }
}

/// Visitor to record the fields from the event record.
struct EventVisitor<'a, LR: LogRecord> {
    log_record: &'a mut LR,
}

impl<'a, LR: LogRecord> EventVisitor<'a, LR> {
    fn new(log_record: &'a mut LR) -> Self {
        EventVisitor { log_record }
    }
}

impl<'a, LR: LogRecord> tracing::field::Visit for EventVisitor<'a, LR> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.log_record.set_body(format!("{:?}", value).into());
        } else {
            self.log_record
                .add_attribute(Key::new(field.name()), AnyValue::from(format!("{value:?}")));
        }
    }

    fn record_str(&mut self, field: &tracing_core::Field, value: &str) {
        //TODO: Consider special casing "message" to populate body and document
        // to users to use message field for log message, to avoid going to the
        // record_debug, which has dyn dispatch, string allocation and
        // formatting cost.

        //TODO: Fix heap allocation. Check if lifetime of &str can be used
        // to optimize sync exporter scenario.
        self.log_record
            .add_attribute(Key::new(field.name()), AnyValue::from(value.to_owned()));
    }

    fn record_bool(&mut self, field: &tracing_core::Field, value: bool) {
        self.log_record
            .add_attribute(Key::new(field.name()), AnyValue::from(value));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.log_record
            .add_attribute(Key::new(field.name()), AnyValue::from(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.log_record
            .add_attribute(Key::new(field.name()), AnyValue::from(value));
    }

    // TODO: Remaining field types from AnyValue : Bytes, ListAny, Boolean
}

/// Visitor to record the fields from the event record.
struct SpanVisitor<'a> {
    extension_values: &'a mut String,
}

impl<'a> SpanVisitor<'a> {
    fn new(extension_values: &'a mut String) -> Self {
        SpanVisitor { extension_values }
    }
}

impl tracing::field::Visit for SpanVisitor<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.extension_values
            .push_str(&format!(", {}: '{:?}'", field.name(), value))
    }

    fn record_str(&mut self, field: &tracing_core::Field, value: &str) {
        self.extension_values
            .push_str(&format!(", {}: '{}'", field.name(), value))
    }

    fn record_bool(&mut self, field: &tracing_core::Field, value: bool) {
        self.extension_values
            .push_str(&format!(", {}: '{}'", field.name(), value))
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.extension_values
            .push_str(&format!(", {}: '{}'", field.name(), value))
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.extension_values
            .push_str(&format!(", {}: '{}'", field.name(), value))
    }

    // TODO: Remaining field types from AnyValue : Bytes, ListAny, Boolean
}
