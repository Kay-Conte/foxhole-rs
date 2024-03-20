use std::sync::{Arc, RwLock};

use foxhole::{
    action::Html,
    connection::Http1,
    resolve::{Endpoint, Get, Query},
    type_cache::{TypeCache, TypeCacheKey},
    Scope, App, sys
};

pub struct Counter(u32);

impl TypeCacheKey for Counter {
    // This must be an `Arc` otherwise the object will be cloned by the query and changes will
    // not persist
    type Value = Arc<RwLock<Counter>>;
}

// The value stored inside `Query` is `Counter::Value`
fn get(_get: Get, counter: Query<Counter>, _e: Endpoint) -> Html {
    counter.0.write().unwrap().0 += 1;

    let page = format!(
        "<h1>This page has been visited {} times!</h1>",
        counter.0.read().unwrap().0
    );

    Html(page)
}

fn main() {
    let scope = Scope::new(sys![get]);

    let mut cache = TypeCache::new();

    cache.insert::<Counter>(Arc::new(RwLock::new(Counter(0))));

    println!("Try connecting with a browser at 'http://localhost:8080'");

    App::builder(scope)
        .cache(cache)
        .run::<Http1>("0.0.0.0:8080");
}
