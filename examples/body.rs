use foxhole::{
    App, Http1,
    Method::Post,
    Router,
};

fn post(_body: &[u8], body_str: &str) -> u16 {
    println!("Body: {}", body_str);

    200
}

fn main() {
    let router = Router::new().add_route("/", Post(post));

    App::builder(router).run::<Http1>("127.0.0.1:8080");
}
