<div align="center">
  <h1>Foxhole</h1>
  <p>
    <strong>A Synchronous HTTP framework for Rust</strong>
  </p>
  <p>

![Minimum Supported Rust Version](https://img.shields.io/badge/rustc-1.65+-ab6000.svg)
[![Crates.io](https://img.shields.io/crates/v/foxhole.svg)](https://crates.io/crates/foxhole)
[![Docs.rs](https://docs.rs/foxhole/badge.svg)](https://docs.rs/foxhole)
![Code Size](https://img.shields.io/github/languages/code-size/Kay-Conte/foxhole-rs)
![Maintained](https://img.shields.io/maintenance/yes/2023?style=flat-square)
[![License](https://img.shields.io/crates/l/foxhole.svg)](https://opensource.org/licenses/MIT)

  </p>
</div>
 
Foxhole is a simple, fast, synchronous framework built for finishing your projects.
 
# Features
- Blazing fast performance (~600k req/sec on a ryzen 7 5700x with `wrk`)
- Built-in threading system that allows you to efficiently handle requests.
- Absolutely no async elements, improving ergonomics.
- Minimal build size, 500kb when stripped.
- Uses `http` a model library you may already be familiar with
- Magic function handlers! See [Getting Started](#getting-started)
- Unique routing system
 
# Getting Started
Foxhole uses a set of handler systems and routing modules to handle requests and responses.   
Here's a starting example of a Hello World server.
```rust
use foxhole::{systems::Html, run, sys, Get, Route};

fn get(_get: Get) -> Html {
    Html("<h1> Foxhole </h1>".to_string())
}

fn main() {
    let router = Route::new(sys![get]);

    // run("127.0.0.1:8080", router);
} 
```

Let's break this down into its components.

## Routing

The router will step through the page by its parts, first starting with the route. It will try to run **all** systems of every node it steps through. Once a response is received it will stop stepping over the request. 

lets assume we have the router `Route::new(sys![auth]).route("page", Route::new(sys![get_page]))` and the request `/page`

In this example, we will first call `auth` if auth returns a response, say the user is not authorized and we would like to respond early, then we stop there. Otherwise we continue to the next node `get_page`

If no responses are returned the server will automatically return `404`. This will be configuarable in the future.

## Parameters/Guards

Function parameters can act as both getters and guards in `foxhole`. 

In the example above, `Get` acts as a guard to make sure the system is only run on `GET` requests. 

Any type that implements the trait `Resolve` is viable to use as a parameter. 

`foxhole` will try to provide the most common guards and getters you will use but few are implemented currenty.

### Example
```rust
use foxhole::{http::Method, PathIter, RequestState, Resolve, ResolveGuard};

pub struct Get;

impl<'a> Resolve<'a> for Get {
    type Output = Self;

    fn resolve(ctx: &'a RequestState, _path_iter: &mut PathIter) -> ResolveGuard<Self::Output> {
        if ctx.request.method() == Method::GET {
            ResolveGuard::Value(Get)
        } else {
            ResolveGuard::None
        }
    }
}
```

## Return types

Systems are required to return a value that implements `MaybeIntoResponse`. 

Additionally note the existence of `IntoResponse` which auto impls `MaybeIntoResponse` for any types that *always* return a response. 

If a type returns `None` out of `MaybeIntoResponse` a response will not be sent and routing will continue to further nodes.

### Example
```rust
use foxhole::{http::Version, IntoResponse, Response};


pub struct Html(pub String);

impl IntoResponse for Html {
    fn response(self) -> Response<Vec<u8>> {
        let bytes = self.0.into_bytes();

        Response::builder()
            .version(Version::HTTP_11)
            .status(200)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Content-Length", format!("{}", bytes.len()))
            .body(bytes)
            .unwrap()
    }
}
```
 
# Contributing
Feel free to open an issue or pull request if you have suggestions for features or improvements!
 
# License
MIT license (LICENSE or https://opensource.org/licenses/MIT)
