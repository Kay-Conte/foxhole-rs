use http::HeaderValue;

use crate::{Request, Response};

pub type BoxLayer<T> = Box<dyn 'static + Layer<T> + Send + Sync>;

/// A collection of `Layers` to be executed in sequence.
#[derive(Default)]
pub struct LayerGroup<I> {
    layers: Vec<BoxLayer<I>>,
}

impl<I> LayerGroup<I> {
    /// Constructs a new `LayerGroup`
    pub fn new() -> Self {
        LayerGroup { layers: Vec::new() }
    }

    /// pushes a new `Layer` to the stack
    pub fn add_layer(mut self, layer: impl 'static + Layer<I> + Send + Sync) -> Self {
        self.layers.push(Box::new(layer));
        self
    }
}

/// A trait providing middleware behaviour on `Request`s and `Response`s
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

/// Default layers for `Response`s
pub struct DefaultResponseGroup;

impl DefaultResponseGroup {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> LayerGroup<Response> {
        let group = LayerGroup::new()
            .add_layer(SetContentLength)
            .add_layer(SetServer);

        #[cfg(feature = "date")]
        let group = group.add_layer(SetDate);

        group
    }
}

/// Sets the content length header of all outgoing requests that may be missing it.
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

pub struct SetServer;

impl Layer<Response> for SetServer {
    fn execute(&self, data: &mut Response) {
        data.headers_mut().insert(
            "server",
            HeaderValue::from_str("foxhole")
                .expect("Failed to convert server name to header value"),
        );
    }
}

/// Sets the date header of all outgoing requests
#[cfg(feature = "date")]
pub struct SetDate;

#[cfg(feature = "date")]
impl Layer<Response> for SetDate {
    fn execute(&self, data: &mut Response) {
        let date = chrono::Utc::now().to_rfc2822();

        let value = HeaderValue::from_str(&date).expect("Failed to convert date to header value");

        data.headers_mut().insert("date", value);
    }
}
