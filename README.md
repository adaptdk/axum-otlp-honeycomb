# Axum OTLP for Honeycomb

Creates a connection from an Axum server to a Honeycomb collector.

## Add to your code

Do the following to add the crates to your Cargo.toml:

```
cargo add axum-otlp-honeycomb --git https://github.com/adaptdk/axum-otlp-honeycomb.git --branch axum-metrics
```

### tracing_subscriber

Where you create your tracing_subscriber do this:
```ignore
use axum_otlp_honeycomb::{init_otlp_layer, init_otlp_log_layer, init_otlp_metrics_layer, HoneycombParams};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::prelude::*;
use std::env;
...

struct OtelGuard {
    trace_provider: Option<SdkTraceProvider>,
    metrics_provider: Option<SdkMetricsProvider>,
}

fn init_tracing_subscriber() {
// Build the Honeycomb Params
let honeycomb_params = HoneycombParams::new(env::var("HONEYCOMB_API_KEY")?)
    .with_service_name(clap::crate_name!().to_string())
    .with_service_version(clap::crate_version!());

// Collect 1% of traces on production, all traces elsewhere:
let sample_rate = if branch == "main" { 0.01 } else { 1.0 };

let tracer_provider = init_otlp_trace_provider(sample_rate, &honeycomb_params);
let log_provider = init_otlp_log_layer(&honeycomb_params);
let metrics_provider = init_otlp_metrics_provider(&honeycomb_params);


tracing_subscriber::Registry::default()
    .with(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_filter(EnvFilter::from_default_env()),
    )
    .with(init_otlp_layer(sample_rate, &honeycomb_params).with_filter(LevelFilter::INFO))
    .with(init_otlp_log_layer(&honeycomb_params).with_filter(LevelFilter::INFO))
    .with(init_otlp_metrics_layer(&honeycomb_params))
    .init();

OtelGuard {
trace_provider
}
```
The first `.with` is for local logging to e.g. Platform.sh's `app.log`. The log-level
is set by the `RUST_LOG` environment variable.

The second `.with` is for tracing to Honeycomb. `LevelFilter::INFO` ensures that tracing
done with `#[tracing::instrument]` is forwarded. Logging with `LevelFilter::DEBUG` produces
far too many traces.

The third `.with` is for forwarding events to Honeycomb's Logs. Again the `LevelFilter::INFO`
ensures that only relevant events are forwarded.

**NOTE**: Any event field named **`body`** will overwrite the event message.

The fourth `.with` is for forwarding metrics to Honeycomb.

### Add layers to Axum app

In your app add this:
```
use axum_otlp_honeycomb::{opentelemetry_tracing_layer, opentelemetry_metrics_layer};

...

    let app = Router::new()
    ...
    .layer(opentelemetry_tracing_layer())
    .layer(opentelemetry_metrics_layer(&honeycomb_params));
```
Or, if you don't want to extract the parent context:
```
    .layer(opentelemetry_tracing_layer_without_parent())
    .layer(opentelemetry_metrics_layer(&honeycomb_params));
```

#### Headers

In the traces sent to Honeycomb the following headers will be removed:
* authorization
* cookie
* and any header whose name contains 'token'

#### User ID

Additionally, a field `user.id` is created in the root span, to allow authorization code to
record the user ID, e.g.:
```
let user_id = get_current_user_id();
Span::current().record("user.id", user_id);
```
Note that the current span needs to be the root span for the server,
otherwise the `record()` call will fail silently.

## Tracing client requests with reqwest

This is done using the `reqwest-tracing` crate:
```
cargo add reqwest-tracing
cargo add reqwest-middleware --features json
```
And then you add the middleware-layer to the reqwest-client:

Change:
```
let client: Client = Client::builder()
    ...
    build()?;
```
to:
```
let reqwest_client: Client = Client::builder()
    ...
    build()?;
let client = reqwest_middleware::ClientBuilder::new(reqwest_client)
    .with(tracing_middleware)
    .build();
```
And change all occurrences of `Client` to `ClientWithMiddleware` and
of `reqwest::Client` to `reqwest_middleware::ClientWithMiddleware`.


## Issues

Trace propagation does not work at the moment. This is a work in progress.
