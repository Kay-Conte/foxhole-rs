use foxhole::{
    Http1,
    resolve::{Endpoint, Get, UrlCollect, UrlPart},
    sys, App, IntoRawBytes, IntoResponse, Scope,
};

pub struct User(String);

impl IntoResponse for User {
    fn response(self) -> foxhole::action::RawResponse {
        let bytes = self.0.into_raw_bytes();

        http::Response::builder()
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
    let scope = Scope::empty()
        .route("user", sys![user])
        .route("chained", sys![collect]);

    println!("Try connecting on a browser at 'http://localhost:8080/user/USERNAME'");

    App::builder(scope).run::<Http1>("0.0.0.0:8080");
}
