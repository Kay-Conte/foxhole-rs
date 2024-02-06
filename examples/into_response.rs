use foxhole::{
    resolve::{Endpoint, Get},
    run, sys, IntoResponse, Response, Route, connection::Http1,
};

// This is a reimplementation of the provided `Html` type.
struct Html(String);

impl IntoResponse for Html {
    fn response(self) -> Response<Vec<u8>> {
        let bytes = self.0.into_bytes();

        Response::builder()
            .status(200)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Content-Length", format!("{}", bytes.len()))
            .body(bytes)
            .unwrap()
    }
}

fn page(_get: Get, _e: Endpoint) -> Html {
    Html("<h1> Hey Friend </h1>".to_string())
}

fn favicon(_get: Get, _e: Endpoint) -> u16 {
    println!("No favicon yet :C");
    404
}

fn main() {
    let router = Route::empty()
        .route("favicon.ico", sys![favicon])
        .route("page", sys![page]);

    println!("Try connecting from a browser at 'http://localhost:8080/page'");

    run::<Http1>("127.0.0.1:8080", router);
}
