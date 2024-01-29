#![doc = include_str!("../README.md")]

pub mod action;
pub mod framework;
pub mod http_utils;
pub mod macros;
pub mod resolve;
pub mod routing;
pub mod systems;
pub mod type_cache;
pub mod connection;

mod lazy;
mod sequential_writer;
mod tasks;

pub use tasks::PathIter;

pub use action::{Action, IntoResponse};
pub use framework::run;
pub use routing::Route;
pub use tasks::RequestState;

pub use http;
pub use http::{Request, Response};
