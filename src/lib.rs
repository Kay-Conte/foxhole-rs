//! [![Build Status](https://travis-ci.org/turnip-rs/vegemite.svg?branch=master)](https://travis-ci.org/turnip-rs/vegemite)
//! [![Crates.io](https://img.shields.io/crates/v/vegemite.svg)](https://crates.io/crates/vegemite)
//! [![Docs.rs](https://docs.rs/vegemite/badge.svg)](https://docs.rs/vegemite)
//! 
//! # Vegemite
//! A Simple, Fast, and Flexible HTTP framework for Rust, Aimed to help you finish your projects.
//! 
//! MSRV: stable (1.65)
//! 
//! # Features
//! - Blazing fast performance, greater than [axum](https://github.com/tokio-rs/axum) and [actix](https://github.com/) for non keep-alive requests.
//! - Built-in threading system that allows you to efficiently handle requests.
//! - Absolutely no async elements, improving ergonomics.
//! - Minimal build size, 500kb when stripped.
//! 
//! # Getting Started
//! ```toml
//! [dependencies]
//! vegemite = "0.1.0"
//! ```
//! 
//! vegemite uses a set of handler systems and routing modules to handle requests and responses. 
//! Here's a starting example of a Hello World server.
//! ```rust
//! use vegemite::{run, sys, Get, Route, Response};
//! 
//! fn get(_get: Get) -> Response<String> {
//!     let content = String::from("<h1>Hello World<h1>");
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
//! # Benchmarks
//! These were run on a AMD Ryzen 7 5700X 3.4GHz with 32GB of RAM.  
//! ### Vegemite:
//! ```
//! $ wrk -t12 -c400 -d30s -H"Connection: close" http://localhost:5000
//! Running 10s test @ http://localhost:5000
//!   12 threads and 400 connections
//!   Thread Stats   Avg      Stdev     Max   +/- Stdev
//!     Latency     2.17ms   14.89ms 413.88ms   98.49%
//!     Req/Sec     9.52k     1.51k   18.44k    70.11%
//!   1138609 requests in 10.10s, 89.04MB read
//! Requests/sec: 112731.16
//! Transfer/sec:      8.82MB
//! ```
//! 
//! ### Actix:
//! ```
//! $ wrk -t12 -c400 -d30s -H"Connection: close" http://localhost:5000
//! Running 10s test @ http://localhost:8080
//!   12 threads and 400 connections
//!   Thread Stats   Avg      Stdev     Max   +/- Stdev
//!     Latency     1.96ms    2.23ms  38.95ms   89.66%
//!     Req/Sec     8.89k     1.63k   16.29k    70.19%
//!   1066903 requests in 10.10s, 150.59MB read
//! Requests/sec: 105641.69
//! Transfer/sec:     14.91MB
//! ```
//! 
//! ### Axum: not sure why the errors on "Connection: close"
//! ```
//! $ wrk -t12 -c400 -d30s -H"Connection: close" http://localhost:5000
//! Running 10s test @ http://localhost:8080
//!   12 threads and 400 connections
//!   Thread Stats   Avg      Stdev     Max   +/- Stdev
//!     Latency     3.28ms    1.58ms  57.66ms   94.89%
//!     Req/Sec     8.72k   431.55    14.40k    83.93%
//!   3129533 requests in 30.08s, 411.87MB read
//!   Socket errors: connect 0, read 3129478, write 0, timeout 0
//! Requests/sec: 104027.39
//! Transfer/sec:     13.69MB
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
pub mod tasks;

pub use framework::run;
pub use routing::Route;
pub use systems::{Resolve, ResolveGuard, MaybeIntoResponse, IntoResponse, Get, Post};
pub use tasks::Context;

pub use http::{Response, Request};
pub use http;
