use foxhole::{action::Html, connection::Http1, resolve::Get, routing::Router, run, sys, Scope};

fn get(_get: Get) -> Html {
    Html("<h1> Foxhole </h1>".to_string())
}

fn main() {
    let scope = Scope::new(sys![get]);

    println!("Running on '127.0.0.1:8080'");

    run::<Http1>("127.0.0.1:8080", Router::new(scope));
}
