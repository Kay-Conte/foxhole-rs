use foxhole::{resolve::HeaderMap, App, Http1, Method::Post, Router};

fn post(HeaderMap(headers): HeaderMap) -> u16 {
    let Some(_token) = headers.get("authorization") else {
        return 401;
    };

    // Do something with `token`

    200
}

fn main() {
    let router = Router::new().add_route("/", Post(post));

    let res = App::builder(router).run::<Http1>("127.0.0.1:8080");

    if let Err(e) = res {
        println!("{e:?}");
    };
}
