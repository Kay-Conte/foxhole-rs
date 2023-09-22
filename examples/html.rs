use http::Response;

use vegemite::{run, sys, Get, IntoResponse, Route};

struct Html {
    value: String,
}

impl IntoResponse for Html {
    fn response(self) -> Response<Vec<u8>> {
        let bytes = self.value.into_bytes();

        Response::builder()
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

fn main() {
    let router = Route::empty().route("page", Route::new(sys![page]));

    run("0.0.0.0:5000", router);
}
