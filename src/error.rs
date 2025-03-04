use std::any::Any;

use crate::{action::RawResponse, IntoResponse};

#[derive(Debug)]
pub enum Error {
    NotAuthorized,
    NotFound,

    MalformedRequest,

    InternalServer,

    QueryNotInCache,

    MissingUrlPart,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Error::NotAuthorized => "Not Authorized",
            Error::NotFound => "Not Found",

            Error::MalformedRequest => "Malformed Request",
            Error::InternalServer => "Internal Server Error",

            Error::QueryNotInCache => "Query Not in Cache",

            Error::MissingUrlPart => "Missing Required Url Part",
        };

        f.write_str(s)
    }
}

impl std::error::Error for Error {}

impl IntoResponse for Error {
    fn response(self) -> RawResponse {
        match self {
            Error::NotAuthorized => 401u16,
            Error::MalformedRequest => 400u16,
            Error::NotFound => 404u16,

            Error::InternalServer => 500u16,

            Error::QueryNotInCache => 500u16,
            Error::MissingUrlPart => 404u16,
        }
        .response()
    }
}

pub trait IntoResponseError:
    'static + Any + std::error::Error + IntoResponse + Send + Sync
{
    fn box_response(self: Box<Self>) -> RawResponse;

    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

impl<T> IntoResponseError for T
where
    T: 'static + Any + std::error::Error + IntoResponse + Send + Sync,
{
    fn box_response(self: Box<Self>) -> RawResponse {
        self.response()
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self as _
    }
}

pub trait HandleError {
    fn handle(&self, err: Box<dyn Any>) -> RawResponse;
}

impl<T, A> HandleError for fn(T) -> A
where
    T: IntoResponseError,
    A: IntoResponse,
{
    fn handle(&self, err: Box<dyn Any>) -> RawResponse {
        let e = err.downcast().unwrap();

        (self)(*e).response()
    }
}
