use foxhole::{action::Html, App, Http1, Method::Get, Router};

fn get() -> Html {
    Html(String::from("<h1> Foxhole! </h1>"))
}

fn fallback() -> u16 {
    404
}

fn main() {
    let router = Router::new().add_route("/", Get(get)).fallback(fallback);

    println!("Running on '127.0.0.1:8080'");

    let res = App::builder(router).run::<Http1>("127.0.0.1:8080");

    if let Err(e) = res {
        println!("{e:?}");
    };
}
