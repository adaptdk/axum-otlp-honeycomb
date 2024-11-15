# Axum OTLP for Honeycomb

Creates a connection from an Axum server to a Honeycomb collector.

## Environment variables

The following environment variables are used:

 *  `HONEYCOMB_API_KEY` contains
     the API key for the Honeycomb environment that traces should be sent to
 *  `OTEL_EXPORTER_OTLP_ENDPOINT` contains the endpoint for Honeycomb -
     default is `https://api.eu1.honeycomb.io/`
 *  `OTEL_SERVICE_NAME` contains the service name. Defaults to the package
     name from Cargo.toml

Only `HONEYCOMB_API_KEY` is required.

## Add to your code

Do the following to add the crates to your Cargo.toml:

```
cargo add axum-otlp-honeycomb --git https://github.com/adaptdk/axum-otlp-honeycomb.git
```

### Tracing_subscriber

Where you create your tracing_subscriber do this:
```
use axum_otlp_honeycomb::init_otlp_layer;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt, util::SubscriberInitExt, Layer};

...

tracing_subscriber::Registry::default()
    .with(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_filter(EnvFilter::from_default_env()),
    )
    .with(init_otlp_layer().with_filter(LevelFilter::INFO))
    .init();
```
The first `.with` is for local logging to eg Platform.sh's `app.log`. The log-level
is set by the `RUST_LOG` environment variable.

### Add layers to Axum app

In your app add this:
```
use axum_otlp_honeycomb::{opentelemetry_tracing_layer, response_with_trace_layer};

...

    let app = Router::new()
    ...
    .layer(opentelemetry_tracing_layer())
    .layer(response_with_trace_layer());
```

## Issues

Trace propagation does not work at the moment. This is a work in progress.
