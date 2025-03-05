use http::{Response, Version};

use crate::{
    error::{Error, IntoResponseError},
    http_utils::IntoRawBytes,
    RequestState,
};

#[cfg(feature = "websocket")]
use crate::{connection::BoxedStream, Request};

pub type RawResponse = Response<Vec<u8>>;

/// This is a helper trait to remove the unnecessary `Action` many response types don't need as
/// they are infallible
pub trait IntoResponse {
    fn response(self) -> RawResponse;
}

/// An action to execute on the current request.
pub enum Action {
    /// Respond to the request.
    Respond(RawResponse),
    #[cfg(feature = "websocket")]
    /// Upgrade the connection to a websocket with a custom handler. Unless implementing a specific
    /// websocket version or protocol negotiation, you probably want to use `foxhole::websocket::Upgrade` instead.
    Upgrade(
        fn(&Request) -> crate::Response,
        Box<dyn Fn(BoxedStream, &RequestState)>,
    ),

    Err(Box<dyn IntoResponseError>),
}

impl Action {
    fn err<E>(e: E) -> Action
    where
        E: IntoResponseError,
    {
        Action::Err(Box::new(e))
    }
}

/// All `System`s must return a type implementing `Action`. This trait decides the
/// behaviour of the underlying router.
/// - `Action::None` The router will continue to the fallback.
/// - `Action::Respond` The router will respond.
/// - `Action::Handle` The task will transfer ownership of the stream to the fn.
pub trait IntoAction {
    fn action(self) -> Action;
}

impl<T> IntoAction for T
where
    T: IntoResponse,
{
    fn action(self) -> Action {
        Action::Respond(self.response())
    }
}

impl<T> IntoResponse for Response<T>
where
    T: IntoRawBytes,
{
    fn response(self) -> RawResponse {
        self.map(IntoRawBytes::into_raw_bytes)
    }
}

impl IntoAction for () {
    fn action(self) -> Action {
        Action::err(Error::NotFound)
    }
}

impl<T> IntoAction for Option<T>
where
    T: IntoAction,
{
    fn action(self) -> Action {
        match self {
            Some(v) => v.action(),
            None => Action::err(Error::InternalServer),
        }
    }
}

impl<T, E> IntoAction for Result<T, E>
where
    T: IntoAction,
    E: IntoAction,
{
    fn action(self) -> Action {
        match self {
            Ok(v) => v.action(),
            Err(e) => e.action(),
        }
    }
}

impl IntoResponse for u16 {
    fn response(self) -> RawResponse {
        Response::builder()
            .version(Version::HTTP_11)
            .status(self)
            .header("Content-Type", "text/plain; charset=UTF-8")
            .header("Content-Length", "0")
            .body(Vec::new())
            .expect("Failed to build request")
    }
}

/// Creates a response with the content-type `application/octet-stream`
/// to use for this.
pub struct Raw(pub Vec<u8>);

impl IntoResponse for Raw {
    fn response(self) -> RawResponse {
        Response::builder()
            .version(Version::HTTP_11)
            .status(200)
            .header("Content-Type", "application/octet-stream")
            .header("Content-Length", format!("{}", self.0.len()))
            .body(self.0)
            .unwrap()
    }
}

/// Creates a response with the content-type `text/plain`
pub struct Plain(pub String);

impl IntoResponse for Plain {
    fn response(self) -> RawResponse {
        let bytes = self.0.into_bytes();

        Response::builder()
            .version(Version::HTTP_11)
            .status(200)
            .header("Content-Type", "text/plain; charset=utf-8")
            .header("Content-Length", format!("{}", bytes.len()))
            .body(bytes)
            .unwrap()
    }
}

/// Creates a response with the content-type `text/html`
pub struct Html(pub String);

impl IntoResponse for Html {
    fn response(self) -> RawResponse {
        let bytes = self.0.into_bytes();

        Response::builder()
            .version(Version::HTTP_11)
            .status(200)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Content-Length", format!("{}", bytes.len()))
            .body(bytes)
            .unwrap()
    }
}

/// Creates a response with the content-type `text/css`
pub struct Css(pub String);

impl IntoResponse for Css {
    fn response(self) -> RawResponse {
        let bytes = self.0.into_bytes();

        Response::builder()
            .version(Version::HTTP_11)
            .status(200)
            .header("Content-Type", "text/css; charset=utf-8")
            .header("Content-Length", format!("{}", bytes.len()))
            .body(bytes)
            .unwrap()
    }
}

/// Creates a response with the content-type `text/javascript`
pub struct Js(pub String);

impl IntoResponse for Js {
    fn response(self) -> RawResponse {
        let bytes = self.0.into_bytes();

        Response::builder()
            .version(Version::HTTP_11)
            .status(200)
            .header("Content-Type", "text/javascript; charset=utf-8")
            .header("Content-Length", format!("{}", bytes.len()))
            .body(bytes)
            .unwrap()
    }
}
