use foxhole::{action::Html, routing::Router};

use foxhole::handler::Method::*;

fn get() -> Html {
    Html("Hey friend".to_string())
}

fn post() -> u16 {
    200
}

fn main() {
    let router = Router::new().add_route("/api", (Get(get), Post(post)));
}
