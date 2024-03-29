use foxhole::{action::Html, App, Http1, Method::Get, Resolve, ResolveGuard, Router};

struct Token(String);

impl Resolve for Token {
    type Output<'a> = Self;

    fn resolve(
        ctx: &foxhole::RequestState,
        _captures: &mut foxhole::Captures,
    ) -> ResolveGuard<Self> {
        let Some(v) = ctx.request.headers().get("authorization") else {
            return ResolveGuard::None;
        };

        // You should handle the `Err` case in real code
        ResolveGuard::Value(Token(v.to_str().unwrap().to_string()))
    }
}

fn get(Token(_token): Token) -> Html {
    Html(String::from("<h1> Foxhole! </h1>"))
}

fn main() {
    let scope = Router::new().add_route("/", Get(get));

    println!("Running on '127.0.0.1:8080'");

    App::builder(scope).run::<Http1>("127.0.0.1:8080");
}
