use std::sync::{Arc, RwLock};

use http::Response;
use vegemite::{type_cache::{TypeCacheKey, TypeCache}, systems::Query, Get, Route, sys, framework::run_with_cache, IntoResponse};

struct Html {
    value: String,
}

impl IntoResponse for Html {
    fn response(self) -> Response<Vec<u8>> {
        let bytes = self.value.into_bytes();

        Response::builder()
            .status(200)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Content-Length", format!("{}", bytes.len()))
            .body(bytes)
            .unwrap()
    }
}

pub struct Counter(u32);

impl TypeCacheKey for Counter {
    // This must be an `Arc` otherwise the object will be cloned by the query and changes will
    // not persist
    type Value = Arc<RwLock<Counter>>;
}

// FIXME counter is innacurate because favicon request also increments counter. Create a new
// `Endpoint` guard when internal iterator becomes peekable
fn get(_get: Get, counter: Query<Counter>) -> Html {
    counter.0.write().unwrap().0 += 1;

    let page = format!("<h1>This page has been visited {} times!</h1>", counter.0.read().unwrap().0);

    Html {
        value: page,
    }
}

fn main() {
    let router = Route::new(sys![get]);

    let mut cache = TypeCache::new();

    cache.insert::<Counter>(Arc::new(RwLock::new(Counter(0))));

    run_with_cache("0.0.0.0:8080", router, cache);
}
