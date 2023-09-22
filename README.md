# Vegemite
<!-- [![Build Status](https://travis-ci.org/turnip-rs/vegemite.svg?branch=master)](https://travis-ci.org/turnip-rs/vegemite) -->
<!-- [![Crates.io](https://img.shields.io/crates/v/vegemite.svg)](https://crates.io/crates/vegemite) -->
[![Docs.rs](https://docs.rs/vegemite/badge.svg)](https://docs.rs/vegemite)
[crates-io](https://crates.io/crates/vegemite)

A Simple, Fast, and Flexible HTTP framework for Rust, Aimed to help you finish your projects.

MSRV: stable (1.65)

# Features
- Blazing fast performance, greater than [axum](https://github.com/tokio-rs/axum) for non keep-alive requests.
- built-in threading system that allows you to efficiently handle requests.
- absolutely no async elements, improving ergonomics.
- Minimal build size, 500kb when stripped.

```
$ wrk -t12 -c400 -d30s -H"Connection: close"
Running 30s test @ http://localhost:5000
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     7.05ms   66.37ms   1.66s    98.09%
    Req/Sec     9.53k     1.85k   15.27k    70.95%
  3418531 requests in 30.08s, 267.33MB read
  Socket errors: connect 0, read 1, write 0, timeout 1
Requests/sec: 113654.63
Transfer/sec: 8.89MB 
```

# Getting Started
```toml
[dependencies]
vegemite = "0.1.0"
```

vegemite uses a set of handler systems and routing modules to handle requests and responses. 
Here's a starting example of a Hello World server.
```rust
use vegemite::{run, sys, Get, Route, Response};

fn get(_get: Get) -> Response<String> {
    let content = String::from("<h1>Hello World<h1>");

    Response::builder()
        .status(200)
        .body(content)
        .unwrap()
}

fn main() {
    let router = Route::new(sys![get]);

    run("127.0.0.1:8080", router);
}
```

# Contributing
Feel free to open an issue or pull request if you have suggestions for features or improvements!

# License
MIT license (LICENSE or https://opensource.org/licenses/MIT)

