use foxhole::{action::Html, App, Http1, Method::Get, Router};

fn get() -> Html {
    Html(String::from("<h1> Foxhole! </h1>"))
}

fn main() {
    let scope = Router::new().add_route("/", Get(get));

    println!("Running on '127.0.0.1:8080'");

    App::builder(scope).run::<Http1>("127.0.0.1:8080");
}
