use vegemite::{run, sys, Get, Route};

fn other_get(_get: Get) -> u16 {
    200
}

fn main() {
    let router = Route::new(sys![other_get]);

    println!("Running on '127.0.0.1:5000'. Try connecting using a browser!");

    run("127.0.0.1:5000", router);
}
