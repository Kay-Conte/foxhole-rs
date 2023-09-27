//!
//! A simple vegemite application ready to deploy to https://shuttle.rs/.
//!
//! Use the `shuttle` feature along with `shuttle-runtime` and `tokio` for the following example.
//!
//! To run locally, use `cargo shuttle run` after running `cargo install cargo-shuttle`.
//!
//! To deploy to shuttle, create an environment using `cargo shuttle project start` and deploy using `cargo shuttle deploy`.
//!

use vegemite::{Get, Response};

fn hello(_get: Get) -> Response<String> {
    Response::new(String::from("Hello, World!"))
}

#[shuttle_runtime::main]
async fn main() -> Result<Route, shuttle_runtime::Error> {
    let router = Route::new(sys![hello]);
    Ok(router)
}
