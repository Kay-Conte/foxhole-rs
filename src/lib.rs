//! <div align="center">
//!   <h1>Vegemite</h1>
//!   <p>
//!     <strong>A Synchronous HTTP framework for Rust</strong>
//!   </p>
//!   <p>
//!
//! ![Minimum Supported Rust Version](https://img.shields.io/badge/rustc-1.65+-ab6000.svg)
//! [![Crates.io](https://img.shields.io/crates/v/vegemite.svg)](https://crates.io/crates/vegemite)
//! [![Docs.rs](https://docs.rs/vegemite/badge.svg)](https://docs.rs/vegemite)
//! ![Code Size](https://img.shields.io/github/languages/code-size/Kay-Conte/vegemite-rs)
//! ![Maintained](https://img.shields.io/maintenance/yes/2023?style=flat-square)
//! [![License](https://img.shields.io/crates/l/vegemite.svg)](https://opensource.org/licenses/MIT)
//!
//!   </p>
//! </div>
//!  
//! Vegemite is Simple, Fast, and Aimed at allowing you finish your projects.
//!  
//! # Features
//! - Blazing fast performance, greater than [Axum](https://github.com/tokio-rs/axum) and [Actix](https://github.com/actix/actix-web) (~600k req/sec on a ryzen 7 5700x with `wrk`)
//! - Built-in threading system that allows you to efficiently handle requests.
//! - Absolutely no async elements, improving ergonomics.
//! - Minimal build size, 500kb when stripped.
//! - Uses `http` a model library you may already be familiar with
//! - Magic function handlers! See [Getting Started](#getting-started)
//! - Unique routing system
//!  
//! # Getting Started
//! Add this to your cargo.toml
//! ```toml
//! [dependencies]
//! vegemite = "0.1.0"
//! ```
//!  
//! Vegemite uses a set of handler systems and routing modules to handle requests and responses.   
//! Here's a starting example of a Hello World server.
//! ```rust
//! use vegemite::{run, sys, Get, Route, Response};
//!  
//! fn get(_get: Get) -> Response<String> {
//!     let content = String::from("<h1>Hello World</h1>");
//!  
//!     Response::builder()
//!         .status(200)
//!         .body(content)
//!         .unwrap()
//! }
//!  
//! fn main() {
//!     let router = Route::new(sys![get]);
//!  
//!     run("127.0.0.1:8080", router);
//! }
//! ```
//!
//! Let's break this down into its components.
//!
//! ## Routing
//!
//! The router will step through the page by its parts, first starting with the route. It will try to run **all** systems of every node it steps through. Once a response is received it will stop stepping over the request.
//!
//! lets assume we have the router `Route::new(sys![auth]).route("page", Route::new(sys![get_page]))` and the request `/page`
//!
//! In this example, we will first call `auth` if auth returns a response, say the user is not authorized and we would like to respond early, then we stop there. Otherwise we continue to the next node `get_page`
//!
//! If no responses are returned the server will automatically return `404`. This will be configuarable in the future.
//!
//! ## Parameters/Guards
//!
//! Function parameters can act as both getters and guards in `vegemite`.
//!
//! In the example above, `Get` acts as a guard to make sure the system is only run on `GET` requests.
//!
//! Any type that implements the trait `Resolve<Output = ResolveGuard<Self>>` is viable to use as a parameter.
//!
//! `vegemite` will try to provide the most common guards and getters you will use but few are implemented currenty.
//!
//! ### Example
//! ```rs
//! pub struct Get;
//!
//! impl Resolve for Get {
//!     fn resolve(ctx: &mut Context) -> ResolveGuard<Self> {
//!         if ctx.request.method() == Method::GET {
//!             ResolveGuard::Value(Get)
//!         } else {
//!             ResolveGuard::None
//!         }
//!     }
//! }
//! ```
//!
//! ## Return types
//!
//! Systems are required to return a value that implements `MaybeIntoResponse`.
//!
//! Additionally note the existence of `IntoResponse` which auto impls `MaybeIntoResponse` for any types that *always* return a response.
//!
//! If a type returns `None` out of `MaybeIntoResponse` a response will not be sent and routing will continue to further nodes.
//!
//! ### Example
//! ```rs
//! impl IntoResponse for u16 {
//!     fn response(self) -> RawResponse {
//!         Response::builder()
//!             .version(Version::HTTP_10)
//!             .status(self)
//!             .header("Content-Type", "text/plain; charset=UTF-8")
//!             .header("Content-Length", "0")
//!             .body(Vec::new())
//!             .expect("Failed to build request")
//!     }
//! }
//! ```
//!  
//! # Contributing
//! Feel free to open an issue or pull request if you have suggestions for features or improvements!
//!  
//! # License
//! MIT license (LICENSE or https://opensource.org/licenses/MIT)

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
