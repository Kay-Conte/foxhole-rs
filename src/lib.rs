#![doc = include_str!("../README.md")]

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

mod lazy;
mod sequential_writer;
mod tasks;
mod tls_connection;

use action::RawResponse;
use tasks::BoxedBodyRequest;
pub use tasks::PathIter;

pub use action::{Action, IntoResponse};
pub use framework::App;
pub use routing::Scope;
pub use tasks::RequestState;

pub use http;

pub type Request = BoxedBodyRequest;
pub type Response = RawResponse;
