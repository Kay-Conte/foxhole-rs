use foxhole::{systems::Html, run, sys, Get, Route};

fn get(_get: Get) -> Html {
    Html("<h1> Foxhole </h1>".to_string())
}

fn main() {
    let router = Route::new(sys![get]);

    println!("Running on '127.0.0.1:8080'.");

    run("127.0.0.1:8080", router);
}
