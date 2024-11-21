## v0.2.0
Released 2024-11-21

* Breaking: Added `sample_rate` argument to `init_otlp_layer`.

Sets the fraction of trtaces that are sampled and forwarded to Honeycomb.

* Added `.gitignore`

## v0.1.6
Released 2024-11-20

* Update `opentelemetry`to version 0.27.0

No changes to the API.

* Describe how to add tracing to reqwest clients.

## v0.1.6
Released 2024-11-20

* Use only version 0.26 of the opentelemetry crates.

## v0.1.5
Released 2024-11-19

* Add method to create otlp-layer without extracting the parent trace-context

## v0.1.4
Released 2024-11-18

* Move setting `OTEL_SERVICE_NAME` and `OTEL_EXPORTER_OTLP_ENDPOINT`
from inside `init_otlp_layer` to the caller, so `clap::crate_name!()`
gets the correct value for service.name.

## v0.1.3
Released 2024-11-18

* Filter http.headers to remove authentification headers

* set service name to be the Cargo package name given
  by `clap::crate_name!()`

  The previous code only worked if the server was started
  by `cargo run`.

  This should work in any case.

## v0.1.2
Released 2024-11-17

* Add support for actix

  Adjust all dependency versions to be as loose as possible.

  Actix gave an error for futures-util - version requirements
  prevented a version to be chosen. This change allowed it to build

## v0.1.1
Released 2024-11-16

* Add our own Layer for tracing.

  The layers in tower_http::trace and axum-tracing-opentelemetry
don't fullfill our needs.

  The external API is unchanged.

## v0.1.0
Released 2024-11-13

Opentelemetry tracing for Axum and Honeycomb, Initial commit
