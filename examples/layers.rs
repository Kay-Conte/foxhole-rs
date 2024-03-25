use foxhole::{
    action::Html,
    connection::Http1,
    layers::{DefaultResponseGroup, Layer},
    resolve::Get,
    sys, App, Request, Response, Scope,
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

fn get(_get: Get) -> Html {
    Html("<h1> Foxhole </h1>".to_string())
}

fn main() {
    let scope = Scope::new(sys![get]);

    println!("Running on '127.0.0.1:8080'");

    App::builder(scope)
        .request_layer(Logger)
        .response_layer(DefaultResponseGroup::new().add_layer(Logger))
        .run::<Http1>("127.0.0.1:8080");
}
