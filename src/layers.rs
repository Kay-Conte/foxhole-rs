use http::HeaderValue;

use crate::{Request, Response};

pub struct LayerGroup<I> {
    layers: Vec<Box<dyn 'static + Layer<I> + Send + Sync>>,
}

impl<I> LayerGroup<I> {
    pub fn new() -> Self {
        LayerGroup { layers: Vec::new() }
    }

    pub fn add_layer(&mut self, layer: impl 'static + Layer<I> + Send + Sync) {
        self.layers.push(Box::new(layer))
    }
}

pub trait Layer<I> {
    fn execute(&self, data: &mut I);
}

impl<I> Layer<I> for LayerGroup<I> {
    fn execute(&self, data: &mut I) {
        for layer in &self.layers {
            layer.execute(data)
        }
    }
}

impl Layer<Request> for () {
    fn execute(&self, _data: &mut Request) {}
}

impl Layer<Response> for () {
    fn execute(&self, _data: &mut Response) {}
}

pub struct ResponseGroup {
    layers: Vec<Box<dyn 'static + Layer<Response> + Send + Sync>>,
}

impl Layer<Response> for ResponseGroup {
    fn execute(&self, data: &mut Response) {
        for layer in &self.layers {
            layer.execute(data)
        }
    }
}

pub struct SetContentLength;

impl Layer<Response> for SetContentLength {
    fn execute(&self, data: &mut Response) {
        if data.headers().contains_key("content-length") {
            return;
        }

        let bytes = data.body().len();

        let value = HeaderValue::from_str(&format!("{bytes}"))
            .expect("Failed to parse length as HeaderValue");

        data.headers_mut().insert("content-length", value);
    }
}
