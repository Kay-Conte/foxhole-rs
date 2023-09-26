use http::Response;

use vegemite::{run, sys, Get, IntoResponse, Route};

struct Html {
    value: String,
}

impl IntoResponse for Html {
    fn response(self) -> Response<Vec<u8>> {
        let bytes = self.value.into_bytes();

        Response::builder()
            .status(200)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Content-Length", format!("{}", bytes.len()))
            .body(bytes)
            .unwrap()
    }
}

fn page(_get: Get) -> Html {
    Html {
        value: "<h1> Hey Friend </h1>".to_string(),
    }
}

fn favicon(_get: Get) -> u16 {
    println!("No favicon yet :C");
    404
}

fn main() {
    let router = Route::empty().route("favicon.ico", Route::new(sys![favicon])).route("page", Route::new(sys![page]));

    run("127.0.0.1:5000", router);
}
