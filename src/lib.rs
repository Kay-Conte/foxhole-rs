#![doc = include_str!("../README.md")]

mod lazy;
mod sequential_writer;
mod tasks;
mod tls_connection;

pub mod action;
pub mod connection;
pub mod framework;
pub mod get_as_slice;
pub mod http_utils;
pub mod layers;
pub mod macros;
pub mod resolve;
pub mod routing;
pub mod systems;
pub mod type_cache;

pub mod websocket;

pub use action::{Action, IntoResponse};
pub use connection::Http1;
pub use framework::App;
pub use http_utils::IntoRawBytes;
pub use layers::{DefaultResponseGroup, Layer};
pub use resolve::{Resolve, ResolveGuard};
pub use routing::Scope;
pub use tasks::{PathIter, RequestState};
pub use type_cache::{TypeCache, TypeCacheKey};

pub use http;

/// Request type used by most of `foxhole`
pub type Request = tasks::BoxedBodyRequest;

/// Response type used by most of `foxhole`
pub type Response = action::RawResponse;
