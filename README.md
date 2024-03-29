<div align="center">
  <img width=500 src="https://github.com/Kay-Conte/foxhole-rs/blob/main/fox_hole_logo.png">
  <h1></img>Foxhole</h1>
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
 
# Opinionated decisions
- No async. Binary bloat and poor ergonomics
- Minimal dependencies

# Features
- Blazing fast performance (~600k req/sec on a ryzen 7 5700x with `wrk`) May be outdated.
- Built-in threading system that allows you to efficiently handle requests.
- Minimal build size, ~500kb when stripped.
- Uses `http`, a model library you may already be familiar with.
- Magic function handlers! See [Getting Started](#getting-started).
- Unique powerful routing system
- near Full Http1.1 support
- Https support in the works. Available on the under feature "tls". largely Untested!
- Http2 support coming.

# Getting Started
Foxhole uses a set of handler systems and routing modules to handle requests and responses.   
Here's a starting example of a Hello World server.
```rust
use foxhole::{action::Html, connection::Http1, resolve::Get, App, sys, Scope};

fn get(_get: Get) -> Html {
    Html(String::from("<h1> Foxhole! </h1>"))
}

fn main() {
    let scope = Scope::new(sys![get]);

    println!("Running on '127.0.0.1:8080'");

    #[cfg(test)]
    App::builder(scope)
        .run::<Http1>("127.0.0.1:8080");
} 
```

Let's break this down into its components.

## Routing

The scope tree will step through the url by its parts, first starting with the root. It will try to run **all** systems of every node it steps through in order. Once a response is received it will stop stepping over the url and respond immediately. 

lets assume we have the tree `Scope::new(sys![auth]).route("page", sys![get_page])` and the request `/page`

In this example, the router will first call `auth` at the root of the tree. If `auth` returns a response, say the user is not authorized and we would like to respond early, then we stop there and respond `401`. Otherwise we continue to the next node `get_page`

If no responses are returned by the end of the tree the server will automatically return `404`. This will be configuarable in the future.

## Parameters/Guards

Function parameters can act as both getters and guards in `foxhole`. 

In the example above, `Get` acts as a guard to make sure the system is only run on `GET` requests. 

Any type that implements the trait `Resolve` is viable to use as a parameter. 

`foxhole` will try to provide the most common guards and getters you will use but few are implemented currenty.

### Example
```rust
use foxhole::{http::Method, PathIter, RequestState, resolve::{Resolve, ResolveGuard}};

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

Systems are required to return a value that implements `Action`. 

Additionally note the existence of `IntoResponse` which can be implemented instead for types that are always a response.

If a type returns `None` out of `Action` a response will not be sent and routing will continue to further nodes. This will likely become an extended enum on websocket support.

### Example
```rust
use foxhole::{http::Version, IntoResponse, Response};

pub struct Html(pub String);

impl IntoResponse for Html {
    fn response(self) -> Response {
        let bytes = self.0.into_bytes();

        http::Response::builder()
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
