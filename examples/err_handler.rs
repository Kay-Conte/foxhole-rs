use foxhole::{
    action::{Html, RawResponse},
    error::Error,
    App, Http1, IntoResponse,
    Method::Get,
    Resolve, Router,
};

#[derive(Debug)]
enum CustomErr {
    Bar,
}

impl std::fmt::Display for CustomErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Bar")
    }
}

impl std::error::Error for CustomErr {}

impl IntoResponse for CustomErr {
    fn response(self) -> foxhole::action::RawResponse {
        500u16.response()
    }
}

struct Fallible;

impl Resolve for Fallible {
    type Output<'a> = Fallible;

    fn resolve<'a>(
        _ctx: &'a foxhole::RequestState,
        _captures: &mut foxhole::Captures,
    ) -> std::result::Result<
        Fallible,
        std::boxed::Box<(dyn foxhole::error::IntoResponseError + 'static)>,
    > {
        Err(Box::new(CustomErr::Bar))
    }
}

fn get() -> Html {
    Html(String::from("<h1> Foxhole! </h1>"))
}

fn fallible(_: Fallible) -> u16 {
    200
}

fn foxhole_err(err: Error) -> u16 {
    println!("{:?}", err);

    404
}

fn custom_err(err: CustomErr) -> RawResponse {
    println!("{:?}", err);

    err.response()
}

fn main() {
    let router = Router::new()
        .add_route("/", Get(get))
        .add_route("fallible", Get(fallible))
        .handler(foxhole_err)
        .handler(custom_err);

    println!("Running on '127.0.0.1:8080'");

    App::builder(router).run::<Http1>("127.0.0.1:8080");
}
