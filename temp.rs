#![feature(prelude_import)]
//! <div align="center">
//!   <h1>Vegemite</h1>
//!   <p>
//!     <strong>A Synchronous HTTP framework for Rust</strong>
//!   </p>
//!   <p>
//!
//! ![Minimum Supported Rust Version](https://img.shields.io/badge/rustc-1.65+-ab6000.svg)
//! [![Crates.io](https://img.shields.io/crates/v/vegemite.svg)](https://crates.io/crates/vegemite)
//! [![Docs.rs](https://docs.rs/vegemite/badge.svg)](https://docs.rs/vegemite)
//! ![Code Size](https://img.shields.io/github/languages/code-size/Kay-Conte/vegemite-rs)
//! ![Maintained](https://img.shields.io/maintenance/yes/2023?style=flat-square)
//! [![License](https://img.shields.io/crates/l/vegemite.svg)](https://opensource.org/licenses/MIT)
//!
//!   </p>
//! </div>
//!
//! Vegemite is Simple, Fast, and Aimed at allowing you finish your projects.
//!
//! # Features
//! - Blazing fast performance, greater than [Axum](https://github.com/tokio-rs/axum) and [Actix](https://github.com/) for non keep-alive requests. [#5](/../../issues/5)
//! - Built-in threading system that allows you to efficiently handle requests.
//! - Absolutely no async elements, improving ergonomics.
//!//!  - Minimal build size, 500kb when stripped.
//! - Uses `http` a model library you may already be familiar with
//! - Magic function handlers! See [Getting Started](#getting-started)
//! - Unique routing system
//!
//! # Getting Started
//! Add this to your cargo.toml
//! ```toml
//! [dependencies]
//! vegemite = "0.1.0"
//! ```
//!
//! Vegemite uses a set of handler systems and routing modules to handle requests and responses.
//! Here's a starting example of a Hello World server.
//! ```rust
//! use vegemite::{run, sys, Get, Route, Response};
//!
//! fn get(_get: Get) -> Response<String> {
//!     let content = String::from("<h1>Hello World</h1>");
//!
//!     Response::builder()
//!         .status(200)
//!         .body(content)
//!         .unwrap()
//! }
//!
//! fn main() {
//!     let router = Route::new(sys![get]);
//!
//!     run("127.0.0.1:8080", router);
//! }
//! ```
//!
//! Let's break this down into its components.
//!
//! ## Routing
//!
//! The router will step through the page by its parts, first starting with the route. It will try to run **all** systems of every node it steps through. Once a response is received it will stop stepping over the request.
//!
//! lets assume we have the router `Route::new(sys![auth]).route("page", Route::new(sys![get_page]))` and the request `/page`
//!
//! In this example, we will first call `auth` if auth returns a response, say the user is not authorized and we would like to respond early, then we stop there. Otherwise we continue to the next node `get_page`
//!
//! If no responses are returned the server will automatically return `404`. This will be configuarable in the future.
//!
//! ## Parameters/Guards
//!
//! Function parameters can act as both getters and guards in `vegemite`.
//!
//! In the example above, `Get` acts as a guard to make sure the system is only run on `GET` requests.
//!
//! Any type that implements the trait `Resolve<Output = ResolveGuard<Self>>` is viable to use as a parameter.
//!
//! `vegemite` will try to provide the most common guards and getters you will use but few are implemented currenty.
//!
//! ### Example
//! ```rs
//! pub struct Get;
//!
//! impl Resolve for Get {
//!     type Output = ResolveGuard<Self>;
//!
//!     fn resolve(ctx: &mut Context) -> Self::Output {
//!         if ctx.request.method() == Method::GET {
//!             ResolveGuard::Value(Get)
//!         } else {
//!             ResolveGuard::None
//!         }
//!     }
//! }
//! ```
//!
//! ## Return types
//!
//! Systems are required to return a value that implements `MaybeIntoResponse`.
//!
//! Additionally note the existence of `IntoResponse` which auto impls `MaybeIntoResponse` for any types that *always* return a response.
//!
//! If a type returns `None` out of `MaybeIntoResponse` a response will not be sent and routing will continue to further nodes.
//!
//! ### Example
//! ```rs
//! impl MaybeIntoResponse for u16 {
//!     fn maybe_response(self) -> Option<RawResponse> {
//!         Some(
//!             Response::builder()
//!                 .version(Version::HTTP_10)
//!                 .status(self)
//!                 .header("Content-Type", "text/plain; charset=UTF-8")
//!                 .header("Content-Length", "0")
//!                 .body(Vec::new())
//!                 .expect("Failed to build request"),
//!         )
//!     }
//! }
//! ```
//!
//! # Contributing
//! Feel free to open an issue or pull request if you have suggestions for features or improvements!
//!
//! # License
//! MIT license (LICENSE or https://opensource.org/licenses/MIT)
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
pub mod framework {
    //! This module provides the application entry point.
    use std::{
        net::{TcpListener, ToSocketAddrs},
        sync::{Arc, RwLock},
    };
    use crate::{
        routing::Route, tasks::{ConnectionTask, RequestTask, TaskPool},
        type_cache::TypeCache,
    };
    /// Application entry point. Call this to run your application.
    pub fn run<A>(address: A, router: Route)
    where
        A: ToSocketAddrs,
    {
        run_with_cache(address, router, TypeCache::new())
    }
    /// Application entry point with an initialized cache.
    pub fn run_with_cache<A>(address: A, router: Route, type_cache: TypeCache)
    where
        A: ToSocketAddrs,
    {
        let incoming = TcpListener::bind(address)
            .expect("Could not bind to local address");
        let router = Arc::new(router);
        let type_cache = Arc::new(RwLock::new(type_cache));
        let connection_pool = TaskPool::<ConnectionTask>::default();
        let request_pool = Arc::new(TaskPool::<RequestTask>::default());
        loop {
            let Ok((stream, _addr)) = incoming.accept() else {
                continue;
            };
            let task = ConnectionTask {
                request_pool: request_pool.clone(),
                cache: type_cache.clone(),
                stream,
                router: router.clone(),
            };
            connection_pool.send_task(task);
        }
    }
}
pub mod http_utils {
    //! This module provides http utility traits and functions for parsing and handling Requests and
    //! Responses
    use http::{Request, Response, StatusCode, Version};
    use std::{
        io::{Write, BufReader, Lines},
        net::TcpStream,
    };
    use crate::systems::RawResponse;
    /// Errors while parsing requests.
    pub enum ParseError {
        MalformedRequest,
        InvalidMethod,
        InvalidProtocolVer,
        InvalidRequestParts,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for ParseError {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    ParseError::MalformedRequest => "MalformedRequest",
                    ParseError::InvalidMethod => "InvalidMethod",
                    ParseError::InvalidProtocolVer => "InvalidProtocolVer",
                    ParseError::InvalidRequestParts => "InvalidRequestParts",
                },
            )
        }
    }
    impl std::fmt::Display for ParseError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ParseError::InvalidProtocolVer => {
                    f.write_fmt(format_args!("Invalid Protocol"))
                }
                ParseError::MalformedRequest => {
                    f.write_fmt(format_args!("Malformed Request"))
                }
                ParseError::InvalidMethod => f.write_fmt(format_args!("Invalid Method")),
                ParseError::InvalidRequestParts => {
                    f.write_fmt(format_args!("Invalid Request Parts"))
                }
            }
        }
    }
    impl std::error::Error for ParseError {}
    pub trait VersionExt: Sized {
        /// # Errors
        ///
        /// Returns `Err` if the `&str` isn't a valid version of the HTTP protocol
        fn parse_version(s: &str) -> Result<Self, ParseError>;
        fn to_string(&self) -> String;
    }
    impl VersionExt for Version {
        fn parse_version(s: &str) -> Result<Version, ParseError> {
            Ok(
                match s {
                    "HTTP/0.9" => Version::HTTP_09,
                    "HTTP/1.0" => Version::HTTP_10,
                    "HTTP/1.1" => Version::HTTP_11,
                    "HTTP/2.0" => Version::HTTP_2,
                    "HTTP/3.0" => Version::HTTP_3,
                    _ => return Err(ParseError::InvalidProtocolVer),
                },
            )
        }
        fn to_string(&self) -> String {
            match *self {
                Version::HTTP_09 => "HTTP/0.9".to_string(),
                Version::HTTP_10 => "HTTP/1.0".to_string(),
                Version::HTTP_11 => "HTTP/1.1".to_string(),
                Version::HTTP_2 => "HTTP/2.0".to_string(),
                Version::HTTP_3 => "HTTP/3.0".to_string(),
                _ => ::core::panicking::panic("internal error: entered unreachable code"),
            }
        }
    }
    fn validate_method(method: &str) -> bool {
        match method {
            "GET" | "POST" | "PUT" | "DELETE" | "HEAD" | "OPTIONS" | "CONNECT" | "TRACE"
            | "PATH" => true,
            _ => false,
        }
    }
    pub trait RequestFromBytes<'a> {
        /// # Errors
        ///
        /// Returns `Err` if the bytes are not a valid HTTP request
        fn take_request(
            bytes: &mut Lines<BufReader<TcpStream>>,
        ) -> Result<Request<()>, ParseError>;
    }
    /// the entirety of the header must be valid utf8
    impl<'a> RequestFromBytes<'a> for Request<&'a [u8]> {
        fn take_request(
            lines: &mut Lines<BufReader<TcpStream>>,
        ) -> Result<Request<()>, ParseError> {
            let Some(Ok(line)) = lines.next() else {
                return Err(ParseError::MalformedRequest);
            };
            let mut parts = line.split(' ');
            let Some(method) = parts.next() else {
                return Err(ParseError::MalformedRequest);
            };
            if !validate_method(method) {
                return Err(ParseError::InvalidMethod);
            }
            let Some(uri) = parts.next() else {
                return Err(ParseError::MalformedRequest);
            };
            let Some(version) = parts.next() else {
                return Err(ParseError::MalformedRequest);
            };
            if parts.next().is_some() {
                return Err(ParseError::MalformedRequest);
            }
            let mut req = Request::builder()
                .method(method)
                .uri(uri)
                .version(Version::parse_version(version)?);
            while let Some(Ok(line)) = lines.next() {
                if line.is_empty() {
                    break;
                }
                let h = line.split_once(": ").ok_or(ParseError::MalformedRequest)?;
                if h.1.is_empty() {
                    return Err(ParseError::MalformedRequest);
                }
                req = req.header(h.0, h.1);
            }
            req.body(()).map_err(|_| ParseError::MalformedRequest)
        }
    }
    fn parse_response_line_into_buf<T>(
        buf: &mut Vec<u8>,
        request: &Response<T>,
    ) -> Result<(), std::io::Error> {
        buf.write_fmt(
            format_args!("{0} {1} \r\n", request.version().to_string(), request.status()),
        )?;
        for (key, value) in request.headers() {
            let _ = buf.write(key.as_str().as_bytes())?;
            buf.write_fmt(format_args!(": "))?;
            let _ = buf.write(value.as_bytes())?;
            buf.write_fmt(format_args!("\r\n"))?;
        }
        buf.write_fmt(format_args!("\r\n"))?;
        Ok(())
    }
    impl<T> IntoRawBytes for Response<T>
    where
        T: IntoRawBytes,
    {
        fn into_raw_bytes(self) -> Vec<u8> {
            let mut buf = ::alloc::vec::Vec::new();
            let _ = parse_response_line_into_buf(&mut buf, &self);
            buf.extend_from_slice(self.map(IntoRawBytes::into_raw_bytes).body());
            buf
        }
    }
    pub trait IntoRawBytes {
        fn into_raw_bytes(self) -> Vec<u8>;
    }
    impl IntoRawBytes for () {
        fn into_raw_bytes(self) -> Vec<u8> {
            ::alloc::vec::Vec::new()
        }
    }
    impl IntoRawBytes for Vec<u8> {
        fn into_raw_bytes(self) -> Vec<u8> {
            self
        }
    }
    impl IntoRawBytes for String {
        fn into_raw_bytes(self) -> Vec<u8> {
            self.into_bytes()
        }
    }
    pub trait ResponseExt: Sized {
        fn base(code: StatusCode) -> Response<()>;
        fn empty(code: impl Into<StatusCode>) -> Response<()>;
        fn into_raw_response(self) -> RawResponse;
    }
    impl<T> ResponseExt for Response<T>
    where
        T: IntoRawBytes,
    {
        fn base(code: StatusCode) -> Response<()> {
            Response::builder().status(code).body(()).unwrap()
        }
        fn empty(code: impl Into<StatusCode>) -> Response<()> {
            Response::builder().status(code.into()).body(()).unwrap()
        }
        fn into_raw_response(self) -> RawResponse {
            self.map(IntoRawBytes::into_raw_bytes)
        }
    }
}
pub mod macros {}
pub mod routing {
    use std::collections::HashMap;
    use crate::systems::DynSystem;
    /// A Node in the Router tree.
    pub struct Route {
        children: HashMap<String, Route>,
        systems: Vec<DynSystem>,
    }
    impl Route {
        /// Construct a new `Route`
        pub fn new(systems: Vec<DynSystem>) -> Self {
            Self {
                children: HashMap::new(),
                systems,
            }
        }
        /// Construct an empty `Route`
        pub fn empty() -> Self {
            Route::new(::alloc::vec::Vec::new())
        }
        /// Add a `Route` as a child of this node
        pub fn route(
            mut self,
            path: impl Into<String>,
            route: impl Into<Route>,
        ) -> Self {
            self.children.insert(path.into(), route.into());
            self
        }
        /// Access the list of systems associated with this node
        pub fn systems(&self) -> &[DynSystem] {
            &self.systems
        }
        /// Route to a child of this node by path
        pub fn get_child<'a>(&'a self, path: &str) -> Option<&'a Route> {
            self.children.get(path)
        }
    }
    impl From<Vec<DynSystem>> for Route {
        fn from(value: Vec<DynSystem>) -> Self {
            Route::new(value)
        }
    }
}
pub mod systems {
    use http::{Method, Response, Version};
    use crate::{http_utils::IntoRawBytes, tasks::RequestState, type_cache::TypeCacheKey};
    pub type RawResponse = Response<Vec<u8>>;
    pub trait IntoResponse {
        fn response(self) -> RawResponse;
    }
    pub trait MaybeIntoResponse {
        fn maybe_response(self) -> Option<RawResponse>;
    }
    impl<T> MaybeIntoResponse for T
    where
        T: IntoResponse,
    {
        fn maybe_response(self) -> Option<RawResponse> {
            Some(self.response())
        }
    }
    impl<T> MaybeIntoResponse for Response<T>
    where
        T: IntoRawBytes,
    {
        fn maybe_response(self) -> Option<RawResponse> {
            Some(self.map(IntoRawBytes::into_raw_bytes))
        }
    }
    impl MaybeIntoResponse for () {
        fn maybe_response(self) -> Option<RawResponse> {
            None
        }
    }
    impl<T> MaybeIntoResponse for Option<T>
    where
        T: MaybeIntoResponse,
    {
        fn maybe_response(self) -> Option<RawResponse> {
            self.and_then(MaybeIntoResponse::maybe_response)
        }
    }
    impl<T, E> MaybeIntoResponse for Result<T, E>
    where
        T: MaybeIntoResponse,
        E: MaybeIntoResponse,
    {
        fn maybe_response(self) -> Option<RawResponse> {
            match self {
                Ok(v) => v.maybe_response(),
                Err(e) => e.maybe_response(),
            }
        }
    }
    impl IntoResponse for u16 {
        fn response(self) -> RawResponse {
            Response::builder()
                .version(Version::HTTP_11)
                .status(self)
                .header("Content-Type", "text/plain; charset=UTF-8")
                .header("Content-Length", "0")
                .body(Vec::new())
                .expect("Failed to build request")
        }
    }
    pub struct Html(pub String);
    impl IntoResponse for Html {
        fn response(self) -> Response<Vec<u8>> {
            let bytes = self.0.into_bytes();
            Response::builder()
                .version(Version::HTTP_11)
                .status(200)
                .header("Content-Type", "text/html; charset=utf-8")
                .header(
                    "Content-Length",
                    {
                        let res = ::alloc::fmt::format(format_args!("{0}", bytes.len()));
                        res
                    },
                )
                .body(bytes)
                .unwrap()
        }
    }
    /// `Resolve` is a trait
    pub trait Resolve: Sized {
        type Output;
        fn resolve(ctx: &mut RequestState) -> Self::Output;
    }
    /// `ResolveGuard` is the expected return type of top level `Resolve`able objects. Only types that
    /// return `ResolveGuard` can be used as function parameters
    pub enum ResolveGuard<T> {
        /// Succesful value, run the system
        Value(T),
        /// Don't run this system or any others, respond early with this response
        Respond(RawResponse),
        /// Don't run this system, but continue routing to other systems
        None,
    }
    impl<T> ResolveGuard<T> {
        pub fn map<N>(self, f: fn(T) -> N) -> ResolveGuard<N> {
            match self {
                ResolveGuard::Value(v) => ResolveGuard::Value(f(v)),
                ResolveGuard::Respond(v) => ResolveGuard::Respond(v),
                ResolveGuard::None => ResolveGuard::None,
            }
        }
    }
    /// Get request guard. A system with this as a parameter requires that the method be GET in order
    /// to run.
    pub struct Get;
    impl Resolve for Get {
        type Output = ResolveGuard<Self>;
        fn resolve(ctx: &mut RequestState) -> Self::Output {
            if ctx.request.method() == Method::GET {
                ResolveGuard::Value(Get)
            } else {
                ResolveGuard::None
            }
        }
    }
    /// Get request guard. A system with this as a parameter requires that the method be POST in order
    /// to run.
    pub struct Post;
    impl Resolve for Post {
        type Output = ResolveGuard<Self>;
        fn resolve(ctx: &mut RequestState) -> Self::Output {
            if ctx.request.method() == Method::POST {
                ResolveGuard::Value(Post)
            } else {
                ResolveGuard::None
            }
        }
    }
    /// "Query" a value from the global_cache of the `RequestState` and clone it.
    pub struct Query<K>(
        pub K::Value,
    )
    where
        K: TypeCacheKey;
    impl<K> Resolve for Query<K>
    where
        K: TypeCacheKey,
        K::Value: Clone,
    {
        type Output = ResolveGuard<Self>;
        fn resolve(ctx: &mut RequestState) -> Self::Output {
            match ctx.global_cache.read().unwrap().get::<K>() {
                Some(v) => ResolveGuard::Value(Query(v.clone())),
                None => ResolveGuard::None,
            }
        }
    }
    /// A function with `Endpoint` as a parameter requires that the internal `path_iter` of the
    /// `RequestState` must be empty. This will only run if there are no trailing path parts of the
    /// uri.
    pub struct Endpoint;
    impl Resolve for Endpoint {
        type Output = ResolveGuard<Self>;
        fn resolve(ctx: &mut RequestState) -> Self::Output {
            match ctx.path_iter.peek() {
                Some(v) if !v.is_empty() => ResolveGuard::None,
                _ => ResolveGuard::Value(Endpoint),
            }
        }
    }
    pub trait System<T> {
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse>;
    }
    pub struct DynSystem {
        inner: Box<
            dyn Fn(&mut RequestState) -> Option<RawResponse> + 'static + Send + Sync,
        >,
    }
    impl DynSystem {
        pub fn new<T, A>(system: T) -> Self
        where
            T: System<A> + 'static + Copy + Send + Sync,
        {
            DynSystem {
                inner: Box::new(move |ctx| system.run(ctx)),
            }
        }
        pub fn call(&self, ctx: &mut RequestState) -> Option<RawResponse> {
            (self.inner)(ctx)
        }
    }
    impl<
        RESPONSE,
        A,
        B,
        C,
        D,
        E,
        F,
        G,
        H,
        I,
        J,
        K,
        L,
        M,
        N,
        O,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<
        (
            RESPONSE,
            A,
            B,
            C,
            D,
            E,
            F,
            G,
            H,
            I,
            J,
            K,
            L,
            M,
            N,
            O,
            P,
            Q,
            R,
            S,
            T,
            U,
            V,
            W,
            X,
            Y,
            Z,
        ),
    > for BASE
    where
        BASE: Fn(
            A,
            B,
            C,
            D,
            E,
            F,
            G,
            H,
            I,
            J,
            K,
            L,
            M,
            N,
            O,
            P,
            Q,
            R,
            S,
            T,
            U,
            V,
            W,
            X,
            Y,
            Z,
        ) -> RESPONSE,
        A: Resolve<Output = ResolveGuard<A>>,
        B: Resolve<Output = ResolveGuard<B>>,
        C: Resolve<Output = ResolveGuard<C>>,
        D: Resolve<Output = ResolveGuard<D>>,
        E: Resolve<Output = ResolveGuard<E>>,
        F: Resolve<Output = ResolveGuard<F>>,
        G: Resolve<Output = ResolveGuard<G>>,
        H: Resolve<Output = ResolveGuard<H>>,
        I: Resolve<Output = ResolveGuard<I>>,
        J: Resolve<Output = ResolveGuard<J>>,
        K: Resolve<Output = ResolveGuard<K>>,
        L: Resolve<Output = ResolveGuard<L>>,
        M: Resolve<Output = ResolveGuard<M>>,
        N: Resolve<Output = ResolveGuard<N>>,
        O: Resolve<Output = ResolveGuard<O>>,
        P: Resolve<Output = ResolveGuard<P>>,
        Q: Resolve<Output = ResolveGuard<Q>>,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let A = match A::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let B = match B::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let C = match C::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let D = match D::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let E = match E::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let F = match F::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(
                A,
                B,
                C,
                D,
                E,
                F,
                G,
                H,
                I,
                J,
                K,
                L,
                M,
                N,
                O,
                P,
                Q,
                R,
                S,
                T,
                U,
                V,
                W,
                X,
                Y,
                Z,
            );
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        B,
        C,
        D,
        E,
        F,
        G,
        H,
        I,
        J,
        K,
        L,
        M,
        N,
        O,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<
        (
            RESPONSE,
            B,
            C,
            D,
            E,
            F,
            G,
            H,
            I,
            J,
            K,
            L,
            M,
            N,
            O,
            P,
            Q,
            R,
            S,
            T,
            U,
            V,
            W,
            X,
            Y,
            Z,
        ),
    > for BASE
    where
        BASE: Fn(
            B,
            C,
            D,
            E,
            F,
            G,
            H,
            I,
            J,
            K,
            L,
            M,
            N,
            O,
            P,
            Q,
            R,
            S,
            T,
            U,
            V,
            W,
            X,
            Y,
            Z,
        ) -> RESPONSE,
        B: Resolve<Output = ResolveGuard<B>>,
        C: Resolve<Output = ResolveGuard<C>>,
        D: Resolve<Output = ResolveGuard<D>>,
        E: Resolve<Output = ResolveGuard<E>>,
        F: Resolve<Output = ResolveGuard<F>>,
        G: Resolve<Output = ResolveGuard<G>>,
        H: Resolve<Output = ResolveGuard<H>>,
        I: Resolve<Output = ResolveGuard<I>>,
        J: Resolve<Output = ResolveGuard<J>>,
        K: Resolve<Output = ResolveGuard<K>>,
        L: Resolve<Output = ResolveGuard<L>>,
        M: Resolve<Output = ResolveGuard<M>>,
        N: Resolve<Output = ResolveGuard<N>>,
        O: Resolve<Output = ResolveGuard<O>>,
        P: Resolve<Output = ResolveGuard<P>>,
        Q: Resolve<Output = ResolveGuard<Q>>,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let B = match B::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let C = match C::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let D = match D::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let E = match E::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let F = match F::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(
                B,
                C,
                D,
                E,
                F,
                G,
                H,
                I,
                J,
                K,
                L,
                M,
                N,
                O,
                P,
                Q,
                R,
                S,
                T,
                U,
                V,
                W,
                X,
                Y,
                Z,
            );
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        C,
        D,
        E,
        F,
        G,
        H,
        I,
        J,
        K,
        L,
        M,
        N,
        O,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<
        (
            RESPONSE,
            C,
            D,
            E,
            F,
            G,
            H,
            I,
            J,
            K,
            L,
            M,
            N,
            O,
            P,
            Q,
            R,
            S,
            T,
            U,
            V,
            W,
            X,
            Y,
            Z,
        ),
    > for BASE
    where
        BASE: Fn(
            C,
            D,
            E,
            F,
            G,
            H,
            I,
            J,
            K,
            L,
            M,
            N,
            O,
            P,
            Q,
            R,
            S,
            T,
            U,
            V,
            W,
            X,
            Y,
            Z,
        ) -> RESPONSE,
        C: Resolve<Output = ResolveGuard<C>>,
        D: Resolve<Output = ResolveGuard<D>>,
        E: Resolve<Output = ResolveGuard<E>>,
        F: Resolve<Output = ResolveGuard<F>>,
        G: Resolve<Output = ResolveGuard<G>>,
        H: Resolve<Output = ResolveGuard<H>>,
        I: Resolve<Output = ResolveGuard<I>>,
        J: Resolve<Output = ResolveGuard<J>>,
        K: Resolve<Output = ResolveGuard<K>>,
        L: Resolve<Output = ResolveGuard<L>>,
        M: Resolve<Output = ResolveGuard<M>>,
        N: Resolve<Output = ResolveGuard<N>>,
        O: Resolve<Output = ResolveGuard<O>>,
        P: Resolve<Output = ResolveGuard<P>>,
        Q: Resolve<Output = ResolveGuard<Q>>,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let C = match C::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let D = match D::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let E = match E::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let F = match F::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(
                C,
                D,
                E,
                F,
                G,
                H,
                I,
                J,
                K,
                L,
                M,
                N,
                O,
                P,
                Q,
                R,
                S,
                T,
                U,
                V,
                W,
                X,
                Y,
                Z,
            );
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        D,
        E,
        F,
        G,
        H,
        I,
        J,
        K,
        L,
        M,
        N,
        O,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<
        (RESPONSE, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z),
    > for BASE
    where
        BASE: Fn(
            D,
            E,
            F,
            G,
            H,
            I,
            J,
            K,
            L,
            M,
            N,
            O,
            P,
            Q,
            R,
            S,
            T,
            U,
            V,
            W,
            X,
            Y,
            Z,
        ) -> RESPONSE,
        D: Resolve<Output = ResolveGuard<D>>,
        E: Resolve<Output = ResolveGuard<E>>,
        F: Resolve<Output = ResolveGuard<F>>,
        G: Resolve<Output = ResolveGuard<G>>,
        H: Resolve<Output = ResolveGuard<H>>,
        I: Resolve<Output = ResolveGuard<I>>,
        J: Resolve<Output = ResolveGuard<J>>,
        K: Resolve<Output = ResolveGuard<K>>,
        L: Resolve<Output = ResolveGuard<L>>,
        M: Resolve<Output = ResolveGuard<M>>,
        N: Resolve<Output = ResolveGuard<N>>,
        O: Resolve<Output = ResolveGuard<O>>,
        P: Resolve<Output = ResolveGuard<P>>,
        Q: Resolve<Output = ResolveGuard<Q>>,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let D = match D::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let E = match E::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let F = match F::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(
                D,
                E,
                F,
                G,
                H,
                I,
                J,
                K,
                L,
                M,
                N,
                O,
                P,
                Q,
                R,
                S,
                T,
                U,
                V,
                W,
                X,
                Y,
                Z,
            );
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        E,
        F,
        G,
        H,
        I,
        J,
        K,
        L,
        M,
        N,
        O,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<
        (RESPONSE, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z),
    > for BASE
    where
        BASE: Fn(
            E,
            F,
            G,
            H,
            I,
            J,
            K,
            L,
            M,
            N,
            O,
            P,
            Q,
            R,
            S,
            T,
            U,
            V,
            W,
            X,
            Y,
            Z,
        ) -> RESPONSE,
        E: Resolve<Output = ResolveGuard<E>>,
        F: Resolve<Output = ResolveGuard<F>>,
        G: Resolve<Output = ResolveGuard<G>>,
        H: Resolve<Output = ResolveGuard<H>>,
        I: Resolve<Output = ResolveGuard<I>>,
        J: Resolve<Output = ResolveGuard<J>>,
        K: Resolve<Output = ResolveGuard<K>>,
        L: Resolve<Output = ResolveGuard<L>>,
        M: Resolve<Output = ResolveGuard<M>>,
        N: Resolve<Output = ResolveGuard<N>>,
        O: Resolve<Output = ResolveGuard<O>>,
        P: Resolve<Output = ResolveGuard<P>>,
        Q: Resolve<Output = ResolveGuard<Q>>,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let E = match E::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let F = match F::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(
                E,
                F,
                G,
                H,
                I,
                J,
                K,
                L,
                M,
                N,
                O,
                P,
                Q,
                R,
                S,
                T,
                U,
                V,
                W,
                X,
                Y,
                Z,
            );
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        F,
        G,
        H,
        I,
        J,
        K,
        L,
        M,
        N,
        O,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<(RESPONSE, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)>
    for BASE
    where
        BASE: Fn(
            F,
            G,
            H,
            I,
            J,
            K,
            L,
            M,
            N,
            O,
            P,
            Q,
            R,
            S,
            T,
            U,
            V,
            W,
            X,
            Y,
            Z,
        ) -> RESPONSE,
        F: Resolve<Output = ResolveGuard<F>>,
        G: Resolve<Output = ResolveGuard<G>>,
        H: Resolve<Output = ResolveGuard<H>>,
        I: Resolve<Output = ResolveGuard<I>>,
        J: Resolve<Output = ResolveGuard<J>>,
        K: Resolve<Output = ResolveGuard<K>>,
        L: Resolve<Output = ResolveGuard<L>>,
        M: Resolve<Output = ResolveGuard<M>>,
        N: Resolve<Output = ResolveGuard<N>>,
        O: Resolve<Output = ResolveGuard<O>>,
        P: Resolve<Output = ResolveGuard<P>>,
        Q: Resolve<Output = ResolveGuard<Q>>,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let F = match F::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        G,
        H,
        I,
        J,
        K,
        L,
        M,
        N,
        O,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<(RESPONSE, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)>
    for BASE
    where
        BASE: Fn(G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE,
        G: Resolve<Output = ResolveGuard<G>>,
        H: Resolve<Output = ResolveGuard<H>>,
        I: Resolve<Output = ResolveGuard<I>>,
        J: Resolve<Output = ResolveGuard<J>>,
        K: Resolve<Output = ResolveGuard<K>>,
        L: Resolve<Output = ResolveGuard<L>>,
        M: Resolve<Output = ResolveGuard<M>>,
        N: Resolve<Output = ResolveGuard<N>>,
        O: Resolve<Output = ResolveGuard<O>>,
        P: Resolve<Output = ResolveGuard<P>>,
        Q: Resolve<Output = ResolveGuard<Q>>,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let G = match G::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        H,
        I,
        J,
        K,
        L,
        M,
        N,
        O,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<(RESPONSE, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)>
    for BASE
    where
        BASE: Fn(H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE,
        H: Resolve<Output = ResolveGuard<H>>,
        I: Resolve<Output = ResolveGuard<I>>,
        J: Resolve<Output = ResolveGuard<J>>,
        K: Resolve<Output = ResolveGuard<K>>,
        L: Resolve<Output = ResolveGuard<L>>,
        M: Resolve<Output = ResolveGuard<M>>,
        N: Resolve<Output = ResolveGuard<N>>,
        O: Resolve<Output = ResolveGuard<O>>,
        P: Resolve<Output = ResolveGuard<P>>,
        Q: Resolve<Output = ResolveGuard<Q>>,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let H = match H::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        I,
        J,
        K,
        L,
        M,
        N,
        O,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<(RESPONSE, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE,
        I: Resolve<Output = ResolveGuard<I>>,
        J: Resolve<Output = ResolveGuard<J>>,
        K: Resolve<Output = ResolveGuard<K>>,
        L: Resolve<Output = ResolveGuard<L>>,
        M: Resolve<Output = ResolveGuard<M>>,
        N: Resolve<Output = ResolveGuard<N>>,
        O: Resolve<Output = ResolveGuard<O>>,
        P: Resolve<Output = ResolveGuard<P>>,
        Q: Resolve<Output = ResolveGuard<Q>>,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        J,
        K,
        L,
        M,
        N,
        O,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<(RESPONSE, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE,
        J: Resolve<Output = ResolveGuard<J>>,
        K: Resolve<Output = ResolveGuard<K>>,
        L: Resolve<Output = ResolveGuard<L>>,
        M: Resolve<Output = ResolveGuard<M>>,
        N: Resolve<Output = ResolveGuard<N>>,
        O: Resolve<Output = ResolveGuard<O>>,
        P: Resolve<Output = ResolveGuard<P>>,
        Q: Resolve<Output = ResolveGuard<Q>>,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        K,
        L,
        M,
        N,
        O,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<(RESPONSE, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE,
        K: Resolve<Output = ResolveGuard<K>>,
        L: Resolve<Output = ResolveGuard<L>>,
        M: Resolve<Output = ResolveGuard<M>>,
        N: Resolve<Output = ResolveGuard<N>>,
        O: Resolve<Output = ResolveGuard<O>>,
        P: Resolve<Output = ResolveGuard<P>>,
        Q: Resolve<Output = ResolveGuard<Q>>,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        L,
        M,
        N,
        O,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<(RESPONSE, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE,
        L: Resolve<Output = ResolveGuard<L>>,
        M: Resolve<Output = ResolveGuard<M>>,
        N: Resolve<Output = ResolveGuard<N>>,
        O: Resolve<Output = ResolveGuard<O>>,
        P: Resolve<Output = ResolveGuard<P>>,
        Q: Resolve<Output = ResolveGuard<Q>>,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        M,
        N,
        O,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<(RESPONSE, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE,
        M: Resolve<Output = ResolveGuard<M>>,
        N: Resolve<Output = ResolveGuard<N>>,
        O: Resolve<Output = ResolveGuard<O>>,
        P: Resolve<Output = ResolveGuard<P>>,
        Q: Resolve<Output = ResolveGuard<Q>>,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        N,
        O,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<(RESPONSE, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE,
        N: Resolve<Output = ResolveGuard<N>>,
        O: Resolve<Output = ResolveGuard<O>>,
        P: Resolve<Output = ResolveGuard<P>>,
        Q: Resolve<Output = ResolveGuard<Q>>,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        O,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<(RESPONSE, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE,
        O: Resolve<Output = ResolveGuard<O>>,
        P: Resolve<Output = ResolveGuard<P>>,
        Q: Resolve<Output = ResolveGuard<Q>>,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<(RESPONSE, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE,
        P: Resolve<Output = ResolveGuard<P>>,
        Q: Resolve<Output = ResolveGuard<Q>>,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(P, Q, R, S, T, U, V, W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<(RESPONSE, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE,
        Q: Resolve<Output = ResolveGuard<Q>>,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(Q, R, S, T, U, V, W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<(RESPONSE, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(R, S, T, U, V, W, X, Y, Z) -> RESPONSE,
        R: Resolve<Output = ResolveGuard<R>>,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(R, S, T, U, V, W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<
        RESPONSE,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<(RESPONSE, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(S, T, U, V, W, X, Y, Z) -> RESPONSE,
        S: Resolve<Output = ResolveGuard<S>>,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(S, T, U, V, W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<RESPONSE, T, U, V, W, X, Y, Z, BASE> System<(RESPONSE, T, U, V, W, X, Y, Z)>
    for BASE
    where
        BASE: Fn(T, U, V, W, X, Y, Z) -> RESPONSE,
        T: Resolve<Output = ResolveGuard<T>>,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(T, U, V, W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<RESPONSE, U, V, W, X, Y, Z, BASE> System<(RESPONSE, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(U, V, W, X, Y, Z) -> RESPONSE,
        U: Resolve<Output = ResolveGuard<U>>,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(U, V, W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<RESPONSE, V, W, X, Y, Z, BASE> System<(RESPONSE, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(V, W, X, Y, Z) -> RESPONSE,
        V: Resolve<Output = ResolveGuard<V>>,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(V, W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<RESPONSE, W, X, Y, Z, BASE> System<(RESPONSE, W, X, Y, Z)> for BASE
    where
        BASE: Fn(W, X, Y, Z) -> RESPONSE,
        W: Resolve<Output = ResolveGuard<W>>,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(W, X, Y, Z);
            r.maybe_response()
        }
    }
    impl<RESPONSE, X, Y, Z, BASE> System<(RESPONSE, X, Y, Z)> for BASE
    where
        BASE: Fn(X, Y, Z) -> RESPONSE,
        X: Resolve<Output = ResolveGuard<X>>,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(X, Y, Z);
            r.maybe_response()
        }
    }
    impl<RESPONSE, Y, Z, BASE> System<(RESPONSE, Y, Z)> for BASE
    where
        BASE: Fn(Y, Z) -> RESPONSE,
        Y: Resolve<Output = ResolveGuard<Y>>,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(Y, Z);
            r.maybe_response()
        }
    }
    impl<RESPONSE, Z, BASE> System<(RESPONSE, Z)> for BASE
    where
        BASE: Fn(Z) -> RESPONSE,
        Z: Resolve<Output = ResolveGuard<Z>>,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return None,
                ResolveGuard::Respond(r) => return Some(r),
            };
            let r = self(Z);
            r.maybe_response()
        }
    }
    impl<RESPONSE, BASE> System<(RESPONSE,)> for BASE
    where
        BASE: Fn() -> RESPONSE,
        RESPONSE: MaybeIntoResponse,
    {
        #[allow(unused)]
        fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {
            let r = self();
            r.maybe_response()
        }
    }
}
pub mod type_cache {
    use std::{
        collections::HashMap, any::{TypeId, Any},
        sync::{Arc, RwLock},
    };
    type Value = Box<dyn Any + Sync + Send>;
    pub type TypeCacheShared = Arc<RwLock<TypeCache>>;
    pub trait TypeCacheKey: 'static {
        type Value: Send + Sync;
    }
    pub struct TypeCache {
        inner: HashMap<TypeId, Value>,
    }
    #[automatically_derived]
    impl ::core::default::Default for TypeCache {
        #[inline]
        fn default() -> TypeCache {
            TypeCache {
                inner: ::core::default::Default::default(),
            }
        }
    }
    impl TypeCache {
        pub fn new() -> Self {
            Self { inner: HashMap::new() }
        }
        pub fn get<K: TypeCacheKey>(&self) -> Option<&K::Value> {
            self.inner.get(&TypeId::of::<K>()).map(|f| f.downcast_ref().unwrap())
        }
        pub fn insert<K: TypeCacheKey>(
            &mut self,
            value: K::Value,
        ) -> Option<Box<K::Value>> {
            self.inner
                .insert(TypeId::of::<K>(), Box::new(value))
                .map(|f| f.downcast().unwrap())
        }
        pub fn remove<K: TypeCacheKey>(&mut self) -> Option<Box<K::Value>> {
            self.inner.remove(&TypeId::of::<K>()).map(|f| f.downcast().unwrap())
        }
    }
}
mod tasks {
    use std::{
        collections::VecDeque, net::TcpStream, str::Split,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Condvar, Mutex,
        },
        time::Duration, io::{BufReader, BufRead},
        iter::Peekable,
    };
    use http::Request;
    use crate::{
        http_utils::{ParseError, RequestFromBytes, IntoRawBytes},
        routing::Route, type_cache::{TypeCache, TypeCacheShared},
        MaybeIntoResponse, sequential_writer::{SequentialWriter, self},
    };
    const MIN_THREADS: usize = 4;
    const TIMEOUT: u64 = 5;
    pub struct ConnectionTask {
        pub request_pool: Arc<TaskPool<RequestTask>>,
        /// An application global type cache
        pub cache: TypeCacheShared,
        pub stream: TcpStream,
        /// A handle to the applications router tree
        pub router: Arc<Route>,
    }
    pub struct RequestTask {
        pub cache: TypeCacheShared,
        pub request: Request<()>,
        pub writer: SequentialWriter<TcpStream>,
        /// A handle to the applications router tree
        pub router: Arc<Route>,
    }
    pub struct RequestState<'a> {
        pub global_cache: TypeCacheShared,
        pub local_cache: TypeCache,
        pub request: Request<()>,
        pub path_iter: Peekable<Split<'a, &'static str>>,
    }
    struct Shared<Task> {
        /// Pool of tasks that need to be run
        pool: Mutex<VecDeque<Task>>,
        /// Conditional var used to sleep and wake threads
        condvar: Condvar,
        /// Total number of threads currently waiting for a task
        waiting_tasks: AtomicUsize,
    }
    impl<Task> Shared<Task> {
        fn waiting(&self) {
            self.waiting_tasks.fetch_add(1, Ordering::Release);
        }
        fn release(&self) {
            self.waiting_tasks.fetch_sub(1, Ordering::Release);
        }
    }
    pub struct TaskPool<Task> {
        shared: Arc<Shared<Task>>,
        handler: fn(Task),
    }
    impl Default for TaskPool<ConnectionTask> {
        fn default() -> Self {
            Self::new(handle_connection)
        }
    }
    impl Default for TaskPool<RequestTask> {
        fn default() -> Self {
            Self::new(handle_request)
        }
    }
    impl<Task> TaskPool<Task>
    where
        Task: 'static + Send,
    {
        pub fn new(handler: fn(Task)) -> Self {
            let pool = TaskPool {
                shared: Arc::new(Shared {
                    pool: Mutex::new(VecDeque::new()),
                    condvar: Condvar::new(),
                    waiting_tasks: AtomicUsize::new(0),
                }),
                handler,
            };
            for _ in 0..MIN_THREADS {
                pool.spawn_thread(false);
            }
            pool
        }
        /// Spawns a thread on the task pool.
        ///
        /// # Panics
        ///
        /// This function will panic on poisoned `Mutex`. This will
        /// likely remain until there is a graceful shutdown mechanism
        ///
        /// This function can also panic on 0 duration.
        fn spawn_thread(&self, should_cull: bool) {
            let shared = self.shared.clone();
            let handler = self.handler.clone();
            std::thread::spawn(move || {
                loop {
                    let mut pool = shared.pool.lock().unwrap();
                    shared.waiting();
                    if should_cull {
                        let (new, timeout) = shared
                            .condvar
                            .wait_timeout(pool, Duration::from_secs(5))
                            .unwrap();
                        if timeout.timed_out() {
                            shared.release();
                            return;
                        }
                        pool = new;
                    } else {
                        pool = shared.condvar.wait(pool).unwrap();
                    }
                    shared.release();
                    let Some(task) = pool.pop_front() else {
                        continue;
                    };
                    drop(pool);
                    handler(task);
                }
            });
        }
        /// Adds a task to the task pool and spawns a thread if there is none available
        ///
        /// # Panics
        /// This function can panic if the mutex is poisoned. Mutex poisoning will likely remain
        /// unhandled in the foreseeable future until a graceful shutdown mechanism is provided.
        pub fn send_task(&self, task: Task) {
            self.shared.pool.lock().unwrap().push_back(task);
            if self.shared.waiting_tasks.load(Ordering::Acquire) == 0 {
                self.spawn_thread(true);
            }
            self.shared.condvar.notify_one();
        }
    }
    fn handle_request(task: RequestTask) {
        let path = task.request.uri().path().to_owned();
        let mut path_iter = path.split("/").peekable();
        path_iter.next();
        let mut ctx = RequestState {
            global_cache: task.cache.clone(),
            local_cache: TypeCache::new(),
            request: task.request,
            path_iter,
        };
        let mut cursor = task.router.as_ref();
        loop {
            for system in cursor.systems() {
                if let Some(r) = system.call(&mut ctx) {
                    let _ = task.writer.send(&r.into_raw_bytes());
                    return;
                }
            }
            let Some(next) = ctx.path_iter.next() else {
                break;
            };
            if let Some(child) = cursor.get_child(next) {
                cursor = child;
            } else {
                break;
            }
        }
        let _ = task.writer.send(&404u16.maybe_response().unwrap().into_raw_bytes());
    }
    /// # Panics
    /// Panics if the stream is closed early. This will be fixed.
    fn handle_connection(task: ConnectionTask) {
        task.stream
            .set_read_timeout(Some(Duration::from_secs(TIMEOUT)))
            .expect("Shouldn't fail unless duration is 0");
        let mut writer = SequentialWriter::new(
            sequential_writer::State::Writer(task.stream.try_clone().unwrap()),
        );
        let mut reader = BufReader::new(task.stream).lines();
        loop {
            match Request::take_request(&mut reader) {
                Ok(req) => {
                    task.request_pool
                        .send_task(RequestTask {
                            cache: task.cache.clone(),
                            request: req,
                            writer: writer.0,
                            router: task.router.clone(),
                        });
                    writer = SequentialWriter::new(
                        sequential_writer::State::Waiting(writer.1),
                    );
                    return;
                }
                Err(ParseError::MalformedRequest) => {
                    continue;
                }
                Err(_) => {
                    return;
                }
            }
        }
    }
}
mod sequential_writer {
    use std::{io::Write, sync::mpsc::{channel, Receiver, Sender}};
    pub enum State<W> {
        Writer(W),
        Waiting(Receiver<W>),
    }
    /// A synchronization type to order writes to a writer.
    pub struct SequentialWriter<W>
    where
        W: Write + Send,
    {
        state: State<W>,
        next: Sender<W>,
    }
    impl<W> SequentialWriter<W>
    where
        W: Write + Send + Sync,
    {
        pub fn new(state: State<W>) -> (Self, Receiver<W>) {
            let (sender, receiver) = channel();
            (Self { state, next: sender }, receiver)
        }
        /// # Blocks
        ///
        /// This function blocks while waiting to receive the writer handle. This has the potential to
        /// block indefinitely in the case where the `SequentialWriter` is never written to.
        ///
        /// # Panics
        ///
        /// This function should only panic if the previous `Sender` has closed without sending a
        /// writer
        pub fn send(self, bytes: &[u8]) -> std::io::Result<()> {
            let mut writer = match self.state {
                State::Writer(w) => w,
                State::Waiting(r) => {
                    r.recv().expect("Failed to get writer from the receiver")
                }
            };
            writer.write_all(bytes)?;
            writer.flush()?;
            let _ = self.next.send(writer);
            Ok(())
        }
    }
}
pub use framework::run;
pub use routing::Route;
pub use systems::{Resolve, ResolveGuard, MaybeIntoResponse, IntoResponse, Get, Post};
pub use tasks::RequestState;
pub use http;
pub use http::{Response, Request};
