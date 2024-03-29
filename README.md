<div align="center"> <img width=500
src="https://github.com/Kay-Conte/foxhole-rs/blob/main/fox_hole_logo.png">
<h1></img>Foxhole</h1> <p> <strong>A Synchronous HTTP framework for
Rust</strong> </p> <p>

![Minimum Supported Rust
Version](https://img.shields.io/badge/rustc-1.65+-ab6000.svg)
[![Crates.io](https://img.shields.io/crates/v/foxhole.svg)](https://crates.io/crates/foxhole)
[![Docs.rs](https://docs.rs/foxhole/badge.svg)](https://docs.rs/foxhole) ![Code
Size](https://img.shields.io/github/languages/code-size/Kay-Conte/foxhole-rs)
[![License](https://img.shields.io/crates/l/foxhole.svg)](https://opensource.org/licenses/MIT)

</p>

  Foxhole is a simple, fast, synchronous framework built for finishing your
  projects. </div>
 
# Notes Because this framework is purely synchronous and relies on OS threads
for task scheduling, memory can become an issue on much larger scales because
of the system minimum stack size.
 
# Opinionated Decisions
- No async. Binary bloat and poor ergonomics
- Minimal dependencies

# Features
- Blazing fast performance (~600k req/sec on a ryzen 7 5700x with `wrk`) May be
outdated.
- Built-in threading system that allows you to efficiently handle requests.
- Minimal build size, ~500kb when stripped.
- Uses `http`, a model library you may already be familiar with.
- Magic function handlers! See [Getting Started](#getting-started).
- Powerful routing system
- Near full Http1.1 support
- Https support in the works. Available on the under feature "tls". largely
Untested!
- Http2 support coming.

# Getting Started Foxhole uses a set of magic handler systems and traits to
simplify handling requests and responses.   Here's a starting example of a
Hello World server. ```rust use foxhole::{action::Html, App, Http1,
Method::Get, Router};

fn get() -> Html { Html(String::from("<h1> Foxhole! </h1>")) }

fn main() { let router = Router::new().add_route("/", Get(get));

println!("Running on '127.0.0.1:8080'");

#[cfg(test)] App::builder(router).run::<Http1>("127.0.0.1:8080"); } ```

Let's break this down into its components.

## Parameters/Guards

Function parameters can act as both getters and guards preventing a system from
running in `foxhole`. 

Any type that implements the trait `Resolve` is capable of acting as a
parameter.

`foxhole` will try to provide the most common guards and getters you will use
but few are implemented currently.

The following is basic implementation of `Token` getter.

### Example ```rust use foxhole::{Resolve, ResolveGuard, RequestState,
Captures};

struct Token(String);

impl Resolve for Token { type Output<'a> = Self;

fn resolve( ctx: &RequestState, _captures: &mut Captures,) ->
ResolveGuard<Self> { let Some(v) = ctx.request.headers().get("authorization")
else { return ResolveGuard::None; };

// You should handle the `Err` case in real code
ResolveGuard::Value(Token(v.to_str().unwrap().to_string())) } }

fn get(Token(_token): Token) { } ```

## Return types

Systems are required to return a value that implements `Action`. 

Additionally note the existence of `IntoResponse` which can be implemented
instead for types that are always a response.

If a type returns `None` out of `Action` a response will not be sent and
routing will continue to the fallback. On failure of the fallback, a 500 will
be sent to the client.

### Example ```rust use foxhole::{IntoResponse, Response};

// This is a reimplementation of the provided `Html` type. struct Html(String);

impl IntoResponse for Html { fn response(self) -> Response { let bytes =
self.0.into_bytes();

http::Response::builder() .status(200) .header("Content-Type", "text/html;
charset=utf-8") .header("Content-Length", format!("{}", bytes.len()))
.body(bytes) .unwrap() } }

fn page() -> Html { Html("<h1> Hey Friend </h1>".to_string()) } ```
 
# Contributing Feel free to open an issue or pull request if you have
suggestions for features or improvements!
 
# License MIT license (LICENSE or https://opensource.org/licenses/MIT)
