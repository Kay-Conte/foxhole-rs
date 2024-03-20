use foxhole::{action::Html, connection::Http1, resolve::Get, App, sys, Scope};

fn get(_get: Get) -> Html {
    Html(String::from("<h1> Foxhole! </h1>"))
}

fn main() {
    let scope = Scope::new(sys![get]);

    println!("Running on '127.0.0.1:8080'");

    App::builder(scope)
        .run::<Http1>("127.0.0.1:8080");
}
