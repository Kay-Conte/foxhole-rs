use foxhole::{resolve::HeaderMap, App, Http1, Method::Get, Router};

fn get(HeaderMap(headers): HeaderMap) {}

fn main() {
    let router = Router::new().add_route("/", Get(get));

    App::builder(router).run::<Http1>("127.0.0.1:8080")
}
