# Axum OTLP for Honeycomb

Creates a connection from an Axum server to a Honeycomb collector.

## Environment variables

The following environment variables are used:

 *  `HONEYCOMB_API_KEY` contains
     the API key for the Honeycomb environment that traces should be sent to
 *  `OTEL_EXPORTER_OTLP_ENDPOINT` contains the endpoint for Honeycomb -
     could be `https://api.eu1.honeycomb.io/`
 *  `OTEL_SERVICE_NAME` contains the service name.

Only `HONEYCOMB_API_KEY` is required.

## Add to your code

Do the following to add the crates to your Cargo.toml:

```
cargo add axum-otlp-honeycomb --git https://github.com/adaptdk/axum-otlp-honeycomb.git
cargo add clap --features cargo
```

### Tracing_subscriber

Where you create your tracing_subscriber do this:
```
use axum_otlp_honeycomb::init_otlp_layer;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt, util::SubscriberInitExt, Layer};
use std::env;
...

env::set_var(
    "OTEL_SERVICE_NAME",
    env::var("OTEL_SERVICE_NAME").unwrap_or(clap::crate_name!().to_string()),
);
env::set_var(
    "OTEL_EXPORTER_OTLP_ENDPOINT",
    env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or("https://api.eu1.honeycomb.io/".to_string()),
);

// Collect 1% of traces on production, all traces elsewhere:
let sample_rate = if branch == "main" { 0.01 } else { 1.0 };
tracing_subscriber::Registry::default()
    .with(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_filter(EnvFilter::from_default_env()),
    )
    .with(init_otlp_layer(sample_rate).with_filter(LevelFilter::INFO))
    .init();
```
The first `.with` is for local logging to eg Platform.sh's `app.log`. The log-level
is set by the `RUST_LOG` environment variable.

### Add layers to Axum app

In your app add this:
```
use axum_otlp_honeycomb::{opentelemetry_tracing_layer};

...

    let app = Router::new()
    ...
    .layer(opentelemetry_tracing_layer());
```
or, if you don't want to extract the parent context:
```
    .layer(opentelemetry_tracing_layer_without_parent());
```

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
And change all occurencies of `Client` to `ClientWithMiddleware` and
of `reqwest::Client` to `reqwest_middleware::ClientWithMiddleware`.


## Issues

Trace propagation does not work at the moment. This is a work in progress.
