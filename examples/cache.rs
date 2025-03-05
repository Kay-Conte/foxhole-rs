use std::sync::RwLock;

use foxhole::{
    action::Html, resolve::Query, App, Http1, Method::Get, Router, TypeCache, TypeCacheKey,
};

pub struct Counter(u32);

impl TypeCacheKey for Counter {
    // This must be an `Arc` otherwise the object will be cloned by `Query` and changes will
    // not persist
    type Value = RwLock<Counter>;
}

// The value stored inside `Query` is `Counter::Value`
fn get(Query(counter): Query<Counter>) -> Html {
    counter.write().unwrap().0 += 1;

    let page = format!(
        "<h1>This page has been visited {} times!</h1>",
        counter.read().unwrap().0
    );

    Html(page)
}

fn main() {
    let router = Router::new().add_route("/", Get(get));

    let mut cache = TypeCache::new();

    cache.insert::<Counter>(RwLock::new(Counter(0)));

    println!("Try connecting with a browser at 'http://localhost:8080'");

    App::builder(router)
        .cache(cache)
        .run::<Http1>("0.0.0.0:8080");
}
