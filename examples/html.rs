use http::Response;
use vegemite::{
    framework::run,
    routing::Route,
    sys,
    systems::{MaybeIntoResponse, Get}
};

struct Html {
    value: String,
}

impl MaybeIntoResponse for Html {
    fn response(self) -> Option<vegemite::systems::RawResponse> {
        let bytes = self.value.into_bytes();

        Some(Response::builder().header("Content-Type", "text/html; charset=utf-8").header("Content-Length", format!("{}", bytes.len())).body(bytes).unwrap())
    }
}

fn page(_get: Get) -> Html {
    Html { value: "<h1> Hey Friend </h1>".to_string() }
}

fn main() {
    let router = Route::empty().route("page", Route::new(sys![page]));

    run("0.0.0.0:5000", router);
}
