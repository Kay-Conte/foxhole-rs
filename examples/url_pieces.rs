use foxhole::{
    http_utils::IntoRawBytes,
    resolve::{Endpoint, Get, UrlCollect, UrlPart},
    run, sys, IntoResponse, Response, Route,
};

pub struct User(String);

impl IntoResponse for User {
    fn response(self) -> foxhole::action::RawResponse {
        let bytes = self.0.into_raw_bytes();

        Response::builder()
            .status(200)
            .header("Content-Type", "text/html")
            .header("Content-Length", format!("{}", bytes.len()))
            .body(bytes)
            .unwrap()
    }
}

// Having `Endpoint` as a parameter after `UrlPart` ensures that there is only one trailing url
// part in the url` This function will consume the next part regardless so be careful.
fn user(_g: Get, part: UrlPart, _e: Endpoint) -> User {
    User(part.0)
}

fn collect(_g: Get, collect: UrlCollect) -> Option<User> {
    collect.0.into_iter().next().map(|i| User(i))
}

fn main() {
    let router = Route::empty()
        .route("user", sys![user])
        .route("chained", sys![collect]);

    println!("Try connecting on a browser at 'http://localhost:8080/user/USERNAME'");

    run("0.0.0.0:8080", router);
}
