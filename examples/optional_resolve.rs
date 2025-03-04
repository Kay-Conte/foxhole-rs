use foxhole::{action::Html, App, Http1, Method::Get, Resolve, Router};

struct Fallible;

impl Resolve for Fallible {
    type Output<'a> = Self;

    fn resolve(
        _ctx: &foxhole::RequestState,
        _captures: &mut foxhole::Captures,
    ) -> std::result::Result<
        Fallible,
        std::boxed::Box<(dyn foxhole::error::IntoResponseError + 'static)>,
    > {
        Err(Box::new(foxhole::error::Error::NotFound))
    }
}

fn get(_optional: Option<Fallible>) -> Html {
    // The Option catches the failed case of fallible and still runs!
    Html(String::from("<h1> Foxhole! </h1>"))
}

fn main() {
    let router = Router::new().add_route("/", Get(get));

    println!("Running on '127.0.0.1:8080'");

    App::builder(router).run::<Http1>("127.0.0.1:8080");
}
