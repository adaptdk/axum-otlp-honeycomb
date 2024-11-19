//! Layer for making tracing spans
//!
//! The code is a combination of code from tower-http::trace and
//! axum-tracing-opentelemetry

use axum::extract::MatchedPath;
use http::{HeaderMap, HeaderValue, Request, Response};
use opentelemetry::trace::{TraceContextExt, TraceFlags};
use pin_project_lite::pin_project;
use std::{
    collections::HashMap, error::Error, future::Future, pin::Pin, task::Poll, time::Instant,
};
use tracing::{field::Empty, info_span, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// function to create the tracing layer
#[must_use]
#[allow(unused)]
pub fn opentelemetry_tracing_layer() -> AxumOtelLayer {
    AxumOtelLayer {
        extract_parent: true,
    }
}

/// function to create the tracing layer without
/// extraction of parent span
#[must_use]
#[allow(unused)]
pub fn opentelemetry_tracing_layer_without_parent() -> AxumOtelLayer {
    AxumOtelLayer {
        extract_parent: false,
    }
}

/// layer/middleware for axum:
///
/// - propagate `OpenTelemetry` context (`trace_id`,...) to server
/// - create a Span for `OpenTelemetry` (and tracing) on call
///
/// `OpenTelemetry` context is extracted from tracing's span.
#[derive(Default, Debug, Clone)]
pub struct AxumOtelLayer {
    extract_parent: bool,
}

impl<S> tower::Layer<S> for AxumOtelLayer {
    /// The wrapped service
    type Service = AxumOtelService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        AxumOtelService {
            extract_parent: self.extract_parent,
            inner,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AxumOtelService<S> {
    extract_parent: bool,
    inner: S,
}

impl<S, B, B2> tower::Service<Request<B>> for AxumOtelService<S>
where
    S: tower::Service<Request<B>, Response = Response<B2>> + Clone + Send + 'static,
    S::Error: Error + 'static, //fmt::Display + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    /// create the tracing-span and return a future that
    /// handles the request and allows us to "do stuff"
    /// on response
    fn call(&mut self, req: Request<B>) -> Self::Future {
        let start = Instant::now();
        let req = req;
        let span = make_span(&req, self.extract_parent);

        let future = {
            // should this be a call to instrument() instead of enter() ??
            // David P has done this, so it is probably correct
            let _enter = span.enter();
            self.inner.call(req)
        };
        ResponseFuture {
            inner: future,
            span,
            start,
        }
    }
}

/// Create a tracing-span from a Request
fn make_span<B>(req: &Request<B>, extract_parent: bool) -> Span {
    let route = http_route(req);
    let method = req.method().as_str();

    let span = info_span!(
        "HTTP request",
        http.request.method = method,
        http.route = route,
        // network.protocol.version = %http_flavor(req.version()),
        server.address = http_host(req),
        // server.port = req.uri().port(),
        http.client.address = Empty, //%$request.connection_info().realip_remote_addr().unwrap_or(""),
        http.headers = headers(req),
        user_agent.original = user_agent(req),
        http.response.status_code = Empty, // to be set on response
        url.path = req.uri().path(),
        url.query = req.uri().query(),
        otel.name = format!("{method} {route}"),
        otel.kind = ?opentelemetry::trace::SpanKind::Server,
        otel.status_code = Empty, // to be set on response
        trace_id = Empty, // to be set on response
        request_id = Empty, // to be set
        exception.message = Empty, // to be set on response
        user.id = "-", // to be set when user-id is found
    );
    if extract_parent {
        span.set_parent(extract_context(req))
    }
    span
}

/// Get (and filter) request headers
fn headers<B>(req: &Request<B>) -> String {
    let filtered_headers: HeaderMap<HeaderValue> = req
        .headers()
        .iter()
        .filter(|(name, _)| *name != "authorization" && *name != "idtoken")
        .map(|(n, v)| (n.clone(), v.clone()))
        .collect();
    format!("{filtered_headers:#?}")
}

#[inline]
fn http_route<B>(req: &Request<B>) -> &str {
    req.extensions()
        .get::<MatchedPath>()
        .map_or_else(|| "", |mp| mp.as_str())
}

#[inline]
fn http_host<B>(req: &http::Request<B>) -> &str {
    req.headers()
        .get(http::header::HOST)
        .map_or(req.uri().host(), |h| h.to_str().ok())
        .unwrap_or("")
}

#[inline]
fn user_agent<B>(req: &http::Request<B>) -> &str {
    req.headers()
        .get(http::header::USER_AGENT)
        .map_or("", |h| h.to_str().unwrap_or(""))
}

// If remote request has no span data the propagator defaults to an unsampled context
#[must_use]
fn extract_context<B>(req: &Request<B>) -> opentelemetry::Context {
    let mut headers: HashMap<String, String> = HashMap::new();
    for (name, value) in req.headers().iter() {
        headers.insert(
            name.as_str().to_string(),
            value.to_str().unwrap_or_default().to_string(),
        );
    }
    opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(&headers))
}

pin_project! {
    /// Response future for [`Trace`].
    ///
    /// [`Trace`]: super::Trace
    pub struct ResponseFuture<F> {
        #[pin]
        pub(crate) inner: F,
        pub(crate) span: Span,
        pub(crate) start: Instant,
    }
}

/// The future created when the request is started
///
/// Updates the tracing span with the statuscode etc
///
/// TODO: Also tries to propagate the context, ie set
/// the header 'traceparent'
impl<Fut, ResBody, E> Future for ResponseFuture<Fut>
where
    Fut: Future<Output = Result<Response<ResBody>, E>>,
    E: std::error::Error + 'static,
{
    type Output = Result<Response<ResBody>, E>;

    #[allow(unused_mut)]
    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let _guard = this.span.enter();
        let mut result = futures_util::ready!(this.inner.poll(cx));
        update_span_from_response_or_error(this.span, &result);
        // if result.is_ok() {
        //     set_tracing_header(&this.span, result.unwrap().as_ref().headers_mut());
        // }
        Poll::Ready(result)
    }
}

#[allow(unused)]
fn set_tracing_header(span: &Span, headers: &mut HeaderMap) {
    let ctx = span.context();
    let ctx_span = ctx.span();
    let span_context = ctx_span.span_context();
    if span_context.is_valid() {
        let header_value = format!(
            "{:02x}-{}-{}-{:02x}",
            0, // = SUPPORTED_VERSION,
            span_context.trace_id(),
            span_context.span_id(),
            span_context.trace_flags() & TraceFlags::SAMPLED
        );

        HeaderValue::from_str(&header_value).map(|value| headers.insert("traceparent", value));
    }
}

fn update_span_from_response<B>(span: &tracing::Span, response: &http::Response<B>) {
    let status = response.status();
    span.record("http.response.status_code", status.as_u16());

    if status.is_server_error() {
        span.record("otel.status_code", "ERROR");
        // see [http-spans.md#status](https://github.com/open-telemetry/semantic-conventions/blob/v1.25.0/docs/http/http-spans.md#status)
        // Span Status MUST be left unset if HTTP status code was in the 1xx, 2xx or 3xx ranges,
        // unless there was another error (e.g., network error receiving the response body;
        // or 3xx codes with max redirects exceeded), in which case status MUST be set to Error.
        // } else {
        //     span.record("otel.status_code", "OK");
    }
}

fn update_span_from_error<E>(span: &tracing::Span, error: &E)
where
    E: Error,
{
    span.record("otel.status_code", "ERROR");
    //span.record("http.status_code", 500);
    span.record("exception.message", error.to_string());
    error
        .source()
        .map(|s| span.record("exception.message", s.to_string()));
}

fn update_span_from_response_or_error<B, E>(
    span: &tracing::Span,
    response: &Result<http::Response<B>, E>,
) where
    E: Error,
{
    match response {
        Ok(response) => {
            update_span_from_response(span, response);
        }
        Err(err) => {
            update_span_from_error(span, err);
        }
    }
}
