use vegemite::{run, sys, Get, Route};

fn get(_get: Get) -> u16 {
    200
}

fn main() {
    let router = Route::new(sys![get]);

    println!("Running on '127.0.0.1:8080'.");

    run("127.0.0.1:8080", router);
}
