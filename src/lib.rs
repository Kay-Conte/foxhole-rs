#![doc = include_str!("../README.md")]

pub mod framework;
pub mod http_utils;
pub mod macros;
pub mod routing;
pub mod systems;
pub mod type_cache;

mod sequential_writer;
mod tasks;

pub use framework::run;
pub use routing::Route;
pub use systems::{Get, IntoResponse, MaybeIntoResponse, Post, Resolve, ResolveGuard};
pub use tasks::RequestState;

pub use http;
pub use http::{Request, Response};
