use vegemite::{run, sys, Get, Route};
use http::Response;

fn get(_get: Get) -> Response<String> {
    let body = "<h1>Hello World<h1>";

    Response::builder()
        .status(200)
        .body(body.to_string())
        .unwrap()
}

fn main() {
    let addr = "127.0.0.1:8080";
    let router = Route::new(sys![get]);

    run(addr, router);
}
