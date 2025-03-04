use foxhole::{action::Html, App, FoxholeResult, Http1, Method::Get, Resolve, Router};

struct Token(String);

impl Resolve for Token {
    type Output<'a> = Self;

    fn resolve(
        ctx: &foxhole::RequestState,
        _captures: &mut foxhole::Captures,
    ) -> FoxholeResult<Token> {
        let Some(v) = ctx.request.headers().get("authorization") else {
            return Err(Box::new(foxhole::error::Error::NotAuthorized));
        };

        // You should handle the `Err` case in real code
        Ok(Token(v.to_str().unwrap().to_string()))
    }
}

fn get(Token(_token): Token) -> Html {
    Html(String::from("<h1> Foxhole! </h1>"))
}

fn main() {
    let router = Router::new().add_route("/", Get(get));

    println!("Running on '127.0.0.1:8080'");

    App::builder(router).run::<Http1>("127.0.0.1:8080");
}
