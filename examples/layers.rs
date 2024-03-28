use foxhole::{
    action::Html, App, DefaultResponseGroup, Http1, Layer, Method::Get, Request, Response, Router
};

pub struct Logger;

// This implementation will be run before any handling of the request.
impl Layer<Request> for Logger {
    fn execute(&self, data: &mut Request) {
        println!("Request url: {}", data.uri())
    }
}

// This implementation will run right before sending to the client.
impl Layer<Response> for Logger {
    fn execute(&self, data: &mut Response) {
        println!("Response: {:?}", data);
    }
}

fn get() -> Html {
    Html("<h1> Foxhole </h1>".to_string())
}

fn main() {
    let router = Router::new().add_route("/", Get(get));

    println!("Running on '127.0.0.1:8080'");

    App::builder(router)
        .request_layer(Logger)
        .response_layer(DefaultResponseGroup::new().add_layer(Logger))
        .run::<Http1>("127.0.0.1:8080");
}
