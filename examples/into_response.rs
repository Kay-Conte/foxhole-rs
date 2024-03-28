use foxhole::{App, Http1, IntoResponse, Method::Get, Router};

// This is a reimplementation of the provided `Html` type.
struct Html(String);

impl IntoResponse for Html {
    fn response(self) -> http::Response<Vec<u8>> {
        let bytes = self.0.into_bytes();

        http::Response::builder()
            .status(200)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Content-Length", format!("{}", bytes.len()))
            .body(bytes)
            .unwrap()
    }
}

fn page() -> Html {
    Html("<h1> Hey Friend </h1>".to_string())
}

fn favicon() -> u16 {
    println!("No favicon yet :C");
    404
}

fn main() {
    let router = Router::new().add_route("/page", Get(page)).add_route("/favicon", Get(favicon));

    println!("Try connecting from a browser at 'http://localhost:8080/page'");

    App::builder(router).run::<Http1>("127.0.0.1:8080");
}
