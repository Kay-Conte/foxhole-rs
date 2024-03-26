use http::{Response, Version};

use crate::{connection::BoxedStream, http_utils::IntoRawBytes};

pub type RawResponse = Response<Vec<u8>>;

/// This is a helper trait to remove the unnecessary `Option` many response types don't need as
/// they do not affect control flow.
pub trait IntoResponse {
    fn response(self) -> RawResponse;
}

pub enum Action {
    Respond(RawResponse),
    Upgrade(Box<dyn Fn(BoxedStream)>),
    None,
}

/// All `System`s must return a type implementing `Action`. This trait decides the
/// behaviour of the underlying router.
/// - `Action::None` The router will continue to the next system
/// - `Action::Respond` The router will respond immediately. No subsequent systems will be run
/// - `Action::Handle` The task will transfer ownership of the stream to the fn. No subsequent
/// systems will be run
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

impl<T> IntoAction for Response<T>
where
    T: IntoRawBytes,
{
    fn action(self) -> Action {
        Action::Respond(self.map(IntoRawBytes::into_raw_bytes))
    }
}

impl IntoAction for () {
    fn action(self) -> Action {
        Action::None
    }
}

impl<T> IntoAction for Option<T>
where
    T: IntoAction,
{
    fn action(self) -> Action {
        match self {
            Some(v) => v.action(),
            None => Action::None,
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
