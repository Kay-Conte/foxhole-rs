use http::{Response, Version};

use crate::http_utils::IntoRawBytes;

pub type RawResponse = Response<Vec<u8>>;

/// This is a helper trait to remove the unnecessary `Option` many response types don't need as
/// they do not affect control flow.
pub trait IntoResponse {
    fn response(self) -> RawResponse;
}

/// All `System`s must return a type implementing `MaybeIntoResponse`. This trait dictates the
/// expected behaviour of the underlying router. If this method returns `None` the router will
/// continue. If it receives `Some` value, it will respond to the connection and stop routing.
pub trait Action {
    fn action(self) -> Option<RawResponse>;
}

impl<T> Action for T
where
    T: IntoResponse,
{
    fn action(self) -> Option<RawResponse> {
        Some(self.response())
    }
}

impl<T> Action for Response<T>
where
    T: IntoRawBytes,
{
    fn action(self) -> Option<RawResponse> {
        Some(self.map(IntoRawBytes::into_raw_bytes))
    }
}

impl Action for () {
    fn action(self) -> Option<RawResponse> {
        None
    }
}

impl<T> Action for Option<T>
where
    T: Action,
{
    fn action(self) -> Option<RawResponse> {
        self.and_then(Action::action)
    }
}

impl<T, E> Action for Result<T, E>
where
    T: Action,
    E: Action,
{
    fn action(self) -> Option<RawResponse> {
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

/// Creates a response with the content-type `application/x-binary` there may be a better MIME type
/// to use for this.
pub struct Raw(Vec<u8>);

impl IntoResponse for Raw {
    fn response(self) -> RawResponse {
        Response::builder()
            .version(Version::HTTP_11)
            .status(200)
            .header("Content-Type", "application/x-binary")
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
