use vegemite::{run, sys, Get, Route};
use http::Response;

fn get(_get: Get) -> Response<String> {
    let body = "<h1>Hello World</h1>";

    Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .header("Content-Length", format!("{}", body.len()))
        .body(body.to_string())
        .unwrap()
}

fn main() {
    let router = Route::new(sys![get]);

    println!("Running on '127.0.0.1:5000'. Try connecting using a browser!");

    run("127.0.0.1:5000", router);

}
