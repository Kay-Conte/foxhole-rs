use foxhole::{
    resolve::{UrlCollect, UrlPart}, App, Http1, IntoRawBytes, IntoResponse, Method::Get, Router
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

fn user(UrlPart(part): UrlPart) -> User {
    User(part)
}

fn collect(UrlCollect(after): UrlCollect) -> Option<User> {
    Some(User(after.join("")))
}

fn main() {
    let router = Router::new()
        .add_route("/user/:username", Get(user))
        .add_route("/collect/*", Get(collect));

    println!("Try connecting on a browser at 'http://localhost:8080/user/USERNAME'");

    App::builder(router).run::<Http1>("0.0.0.0:8080");
}
