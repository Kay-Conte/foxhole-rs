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
