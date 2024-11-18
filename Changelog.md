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
