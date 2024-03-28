#![feature(prelude_import)]
/*!<div align="center">
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
*/
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
mod lazy {
    use std::{cell::OnceCell, sync::mpsc::{channel, Receiver, Sender}};
    use crate::get_as_slice::GetAsSlice;
    pub struct Lazy<T> {
        receiver: Receiver<T>,
        value: OnceCell<T>,
    }
    impl Lazy<Vec<u8>> {
        /// Constructs a new instance of `Lazy` and returns it's corresponding `Sender`
        pub fn new() -> (Self, Sender<Vec<u8>>) {
            let (sender, receiver) = channel();
            (
                Lazy {
                    receiver,
                    value: OnceCell::new(),
                },
                sender,
            )
        }
        /// This call blocks until the body has been read from the `TcpStream`
        pub fn get(&self) -> &[u8] {
            self.value.get_or_init(|| self.receiver.recv().unwrap())
        }
    }
    impl GetAsSlice for Lazy<Vec<u8>> {
        fn get_as_slice(&self) -> &[u8] {
            self.get()
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
        W: Write,
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
mod tasks {
    use std::{
        collections::VecDeque, iter::Peekable, marker::PhantomData, net::TcpStream,
        str::Split,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Condvar, Mutex,
        },
        time::Duration,
    };
    use http::Request;
    use crate::{
        connection::{Connection, Responder},
        get_as_slice::GetAsSlice, layers::BoxLayer, type_cache::TypeCache, Action,
        IntoResponse, Response, Scope,
    };
    const MIN_THREADS: usize = 4;
    const TIMEOUT: u64 = 5;
    /// Request with a boxed body implementing `GetAsSlice` This is the standard request type
    /// throughout the library
    pub type BoxedBodyRequest = Request<Box<dyn 'static + GetAsSlice + Send>>;
    /// The url iterator type used by the library. This can be found in `RequestState` accessed via the
    /// `Resolve` trait
    pub type PathIter<'a> = Peekable<Split<'a, &'static str>>;
    pub(crate) trait Task {
        fn run(self: Box<Self>);
    }
    pub(crate) struct ConnectionTask<C> {
        pub task_pool: TaskPool,
        /// An application global type cache
        pub cache: Arc<TypeCache>,
        pub stream: TcpStream,
        /// A handle to the applications router tree
        pub router: Arc<Scope>,
        pub request_layer: Arc<BoxLayer<crate::Request>>,
        pub response_layer: Arc<BoxLayer<Response>>,
        pub phantom_data: PhantomData<C>,
    }
    impl<C> Task for ConnectionTask<C>
    where
        C: 'static + Connection,
    {
        fn run(self: Box<Self>) {
            let Ok(mut connection) = C::new(Box::new(self.stream)) else {
                return;
            };
            connection
                .set_timeout(Some(Duration::from_secs(TIMEOUT)))
                .expect("Shouldn't fail unless duration is 0");
            while let Ok((request, responder)) = connection.next_frame() {
                let r = request
                    .map(|i| {
                        let b: Box<dyn 'static + GetAsSlice + Send> = Box::new(i);
                        b
                    });
                let mut should_close: bool = true;
                if let Some(header) = r
                    .headers()
                    .get("connection")
                    .and_then(|i| i.to_str().ok())
                {
                    should_close = header != "keep-alive";
                }
                self.task_pool
                    .send_task(RequestTask::<_, C> {
                        cache: self.cache.clone(),
                        upgrade: None,
                        request: r,
                        responder,
                        router: self.router.clone(),
                        request_layer: self.request_layer.clone(),
                        response_layer: self.response_layer.clone(),
                    });
                if should_close {
                    break;
                }
            }
        }
    }
    pub(crate) struct RequestTask<R, C> {
        pub cache: Arc<TypeCache>,
        pub request: BoxedBodyRequest,
        pub responder: R,
        /// A handle to the applications router tree
        pub router: Arc<Scope>,
        /// This field is only used on `websocket`
        #[allow(dead_code)]
        pub upgrade: Option<C>,
        pub request_layer: Arc<BoxLayer<crate::Request>>,
        pub response_layer: Arc<BoxLayer<Response>>,
    }
    impl<R, C> Task for RequestTask<R, C>
    where
        R: Responder,
        C: Connection,
    {
        fn run(self: Box<Self>) {
            let path = self.request.uri().path().to_owned();
            let mut path_iter = path.split("/").peekable();
            path_iter.next();
            let mut ctx = RequestState {
                global_cache: self.cache.clone(),
                request: self.request,
            };
            self.request_layer.execute(&mut ctx.request);
            let mut cursor = self.router.as_ref();
            loop {
                for system in cursor.systems() {
                    match system.call(&ctx, &mut path_iter) {
                        Action::Respond(mut r) => {
                            self.response_layer.execute(&mut r);
                            let _ = self.responder.respond(r);
                            return;
                        }
                        Action::None => {}
                    }
                }
                let Some(next) = path_iter.next() else {
                    break;
                };
                if let Some(child) = cursor.get_child(next) {
                    cursor = child;
                } else {
                    break;
                }
            }
            let _ = self.responder.respond(404u16.response());
        }
    }
    /// Holds the state of the request handling. This can be accessed via the `Resolve` trait.
    pub struct RequestState {
        pub global_cache: Arc<TypeCache>,
        pub request: BoxedBodyRequest,
    }
    struct Shared {
        /// Pool of tasks that need to be run
        pool: Mutex<VecDeque<Box<dyn Task + Send + 'static>>>,
        /// Conditional var used to sleep and wake threads
        condvar: Condvar,
        /// Total number of threads currently waiting for a task
        waiting_tasks: AtomicUsize,
    }
    impl Shared {
        fn waiting(&self) {
            self.waiting_tasks.fetch_add(1, Ordering::Release);
        }
        fn release(&self) {
            self.waiting_tasks.fetch_sub(1, Ordering::Release);
        }
    }
    pub(crate) struct TaskPool {
        shared: Arc<Shared>,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for TaskPool {
        #[inline]
        fn clone(&self) -> TaskPool {
            TaskPool {
                shared: ::core::clone::Clone::clone(&self.shared),
            }
        }
    }
    impl TaskPool {
        pub fn new() -> Self {
            let pool = TaskPool {
                shared: Arc::new(Shared {
                    pool: Mutex::new(VecDeque::new()),
                    condvar: Condvar::new(),
                    waiting_tasks: AtomicUsize::new(0),
                }),
            };
            for _ in 0..MIN_THREADS {
                pool.spawn_thread(false, None);
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
        fn spawn_thread(
            &self,
            should_cull: bool,
            initial_task: Option<Box<dyn Task + Send>>,
        ) {
            let shared = self.shared.clone();
            std::thread::spawn(move || {
                match initial_task {
                    Some(task) => task.run(),
                    None => {}
                }
                loop {
                    let mut pool = shared.pool.lock().unwrap();
                    shared.waiting();
                    if should_cull {
                        let (new, timeout) = shared
                            .condvar
                            .wait_timeout(pool, Duration::from_secs(5))
                            .unwrap();
                        if timeout.timed_out() {
                            break;
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
                    task.run();
                }
                shared.release()
            });
        }
        /// Adds a task to the task pool and spawns a thread if there is none available
        ///
        /// # Panics
        /// This function can panic if the mutex is poisoned. Mutex poisoning will likely remain
        /// unhandled in the foreseeable future until a graceful shutdown mechanism is provided.
        pub fn send_task<T>(&self, task: T)
        where
            T: Task + Send + 'static,
        {
            if self.shared.waiting_tasks.load(Ordering::Acquire) < MIN_THREADS {
                self.spawn_thread(true, Some(Box::new(task)));
            } else {
                self.shared.pool.lock().unwrap().push_back(Box::new(task));
                self.shared.condvar.notify_one();
            }
        }
    }
}
mod tls_connection {
    use std::{
        io::{Read, Write},
        net::TcpStream, sync::{Arc, RwLock},
    };
    use rustls::ServerConnection;
    pub struct TlsConnection {
        pub stream: TcpStream,
        pub conn: Arc<RwLock<ServerConnection>>,
    }
    impl TlsConnection {
        pub fn new(stream: TcpStream, conn: Arc<RwLock<ServerConnection>>) -> Self {
            Self { stream, conn }
        }
    }
    impl Read for TlsConnection {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.conn.write().unwrap().reader().read(buf)
        }
    }
    impl Write for TlsConnection {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.conn.write().unwrap().writer().write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.conn.write().unwrap().complete_io(&mut self.stream).map(|_| ())
        }
    }
}
pub mod action {
    use http::{Response, Version};
    use crate::http_utils::IntoRawBytes;
    pub type RawResponse = Response<Vec<u8>>;
    /// This is a helper trait to remove the unnecessary `Option` many response types don't need as
    /// they do not affect control flow.
    pub trait IntoResponse {
        fn response(self) -> RawResponse;
    }
    pub enum Action {
        Respond(RawResponse),
        None,
    }
    /// All `System`s must return a type implementing `Action`. This trait decides the
    /// behaviour of the underlying router.
    /// - `Action::None` The router will continue to the next system
    /// - `Action::Respond` The router will respond immediately. No subsequent systems will be run
    /// - `Action::Handle` The task will transfer ownership of the stream to the fn. No subsequent
    /// systems will be run
    pub trait IntoAction {
        fn action(self) -> Action;
    }
    impl<T> IntoAction for T
    where
        T: IntoResponse,
    {
        fn action(self) -> Action {
            Action::Respond(self.response())
        }
    }
    impl<T> IntoAction for Response<T>
    where
        T: IntoRawBytes,
    {
        fn action(self) -> Action {
            Action::Respond(self.map(IntoRawBytes::into_raw_bytes))
        }
    }
    impl IntoAction for () {
        fn action(self) -> Action {
            Action::None
        }
    }
    impl<T> IntoAction for Option<T>
    where
        T: IntoAction,
    {
        fn action(self) -> Action {
            match self {
                Some(v) => v.action(),
                None => Action::None,
            }
        }
    }
    impl<T, E> IntoAction for Result<T, E>
    where
        T: IntoAction,
        E: IntoAction,
    {
        fn action(self) -> Action {
            match self {
                Ok(v) => v.action(),
                Err(e) => e.action(),
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
    /// Creates a response with the content-type `application/x-binary` there may be a better MIME type
    /// to use for this.
    pub struct Raw(Vec<u8>);
    impl IntoResponse for Raw {
        fn response(self) -> RawResponse {
            Response::builder()
                .version(Version::HTTP_11)
                .status(200)
                .header("Content-Type", "application/x-binary")
                .header(
                    "Content-Length",
                    {
                        let res = ::alloc::fmt::format(
                            format_args!("{0}", self.0.len()),
                        );
                        res
                    },
                )
                .body(self.0)
                .unwrap()
        }
    }
    /// Creates a response with the content-type `text/plain`
    pub struct Plain(pub String);
    impl IntoResponse for Plain {
        fn response(self) -> RawResponse {
            let bytes = self.0.into_bytes();
            Response::builder()
                .version(Version::HTTP_11)
                .status(200)
                .header("Content-Type", "text/plain; charset=utf-8")
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
    /// Creates a response with the content-type `text/html`
    pub struct Html(pub String);
    impl IntoResponse for Html {
        fn response(self) -> RawResponse {
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
    /// Creates a response with the content-type `text/css`
    pub struct Css(pub String);
    impl IntoResponse for Css {
        fn response(self) -> RawResponse {
            let bytes = self.0.into_bytes();
            Response::builder()
                .version(Version::HTTP_11)
                .status(200)
                .header("Content-Type", "text/css; charset=utf-8")
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
    /// Creates a response with the content-type `text/javascript`
    pub struct Js(pub String);
    impl IntoResponse for Js {
        fn response(self) -> RawResponse {
            let bytes = self.0.into_bytes();
            Response::builder()
                .version(Version::HTTP_11)
                .status(200)
                .header("Content-Type", "text/javascript; charset=utf-8")
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
}
pub mod connection {
    use std::{
        io::{BufReader, ErrorKind, Read, Write},
        net::TcpStream, sync::mpsc::{Receiver, Sender},
        time::Duration,
    };
    use http::Request;
    use crate::{
        action::RawResponse, get_as_slice::GetAsSlice,
        http_utils::{take_request, IntoRawBytes},
        lazy::Lazy, sequential_writer::{self, SequentialWriter},
        tls_connection::TlsConnection,
    };
    /// A marker used to encapsulate the required traits for a stream used by `Http1`
    pub trait BoxedStreamMarker: Read + Write + BoxedTryClone + SetTimeout + Send + Sync {}
    impl<T> BoxedStreamMarker for T
    where
        T: Read + Write + BoxedTryClone + SetTimeout + Send + Sync,
    {}
    pub type BoxedStream = Box<dyn BoxedStreamMarker>;
    /// A trait providing a `set_timeout` function for streams
    pub trait SetTimeout {
        /// Set the timeout for reads and writes on the current object
        fn set_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()>;
    }
    impl SetTimeout for TcpStream {
        fn set_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()> {
            self.set_read_timeout(timeout)?;
            self.set_write_timeout(timeout)
        }
    }
    impl SetTimeout for TlsConnection {
        fn set_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()> {
            self.stream.set_read_timeout(timeout)?;
            self.stream.set_write_timeout(timeout)
        }
    }
    /// A trait providing a method of cloning for boxed streams
    pub trait BoxedTryClone {
        fn try_clone(&self) -> std::io::Result<BoxedStream>;
    }
    impl BoxedTryClone for TcpStream {
        fn try_clone(&self) -> std::io::Result<BoxedStream> {
            self.try_clone().map(|s| Box::new(s) as BoxedStream)
        }
    }
    impl BoxedTryClone for TlsConnection {
        fn try_clone(&self) -> std::io::Result<BoxedStream> {
            self.stream
                .try_clone()
                .map(|s| {
                    Box::new(TlsConnection::new(s, self.conn.clone())) as BoxedStream
                })
        }
    }
    /// A trait providing necessary functions to handle a connection
    pub trait Connection: Sized + Send {
        type Body: 'static + GetAsSlice + Send;
        type Responder: 'static + Responder;
        fn new(conn: BoxedStream) -> Result<Self, std::io::Error>;
        fn set_timeout(
            &mut self,
            timeout: Option<Duration>,
        ) -> Result<(), std::io::Error>;
        /// Reading of the body of the previous frame may occur on subsequent calls depending on
        /// implementation
        fn next_frame(
            &mut self,
        ) -> Result<(Request<Self::Body>, Self::Responder), std::io::Error>;
        fn upgrade(self) -> BoxedStream;
    }
    /// A trait providing necessary functionality to respond to a connection
    pub trait Responder: Sized + Send {
        /// Write bytes of response to the underlying writer. This can be expected to be the full
        /// response
        fn write_bytes(self, bytes: Vec<u8>) -> Result<(), std::io::Error>;
        fn respond(
            self,
            response: impl Into<RawResponse>,
        ) -> Result<(), std::io::Error> {
            let response: RawResponse = response.into();
            let bytes = response.into_raw_bytes();
            self.write_bytes(bytes)
        }
    }
    /// HTTP 1.1
    pub struct Http1 {
        conn: BoxedStream,
        next_writer: Option<Receiver<BoxedStream>>,
        unfinished: Option<(usize, Sender<Vec<u8>>)>,
    }
    impl Http1 {
        fn next_writer(
            &mut self,
        ) -> Result<SequentialWriter<BoxedStream>, std::io::Error> {
            Ok(
                match self.next_writer.take() {
                    Some(writer) => {
                        let (writer, receiver) = SequentialWriter::new(
                            sequential_writer::State::Waiting(writer),
                        );
                        self.next_writer = Some(receiver);
                        writer
                    }
                    None => {
                        let (writer, receiver) = SequentialWriter::new(
                            sequential_writer::State::Writer(self.conn.try_clone()?),
                        );
                        self.next_writer = Some(receiver);
                        writer
                    }
                },
            )
        }
    }
    impl Connection for Http1 {
        type Body = Lazy<Vec<u8>>;
        type Responder = SequentialWriter<BoxedStream>;
        fn new(conn: BoxedStream) -> Result<Self, std::io::Error> {
            Ok(Self {
                conn,
                next_writer: None,
                unfinished: None,
            })
        }
        fn set_timeout(
            &mut self,
            timeout: Option<Duration>,
        ) -> Result<(), std::io::Error> {
            self.conn.set_timeout(timeout)
        }
        fn next_frame(
            &mut self,
        ) -> Result<(Request<Self::Body>, Self::Responder), std::io::Error> {
            if let Some((len, sender)) = self.unfinished.take() {
                let mut buf = ::alloc::vec::from_elem(0, len);
                if self.conn.read_exact(&mut buf).is_err() {
                    let _ = sender.send(::alloc::vec::Vec::new());
                }
                let _ = sender.send(buf);
            }
            let req = take_request(&mut BufReader::new(&mut self.conn))
                .map_err(|_| {
                    std::io::Error::new(
                        ErrorKind::Other,
                        "Failed to parse request from stream",
                    )
                })?;
            let body_len = req
                .headers()
                .get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            let (lazy, sender) = Lazy::new();
            self.unfinished = Some((body_len, sender));
            Ok((req.map(|_| lazy), self.next_writer()?))
        }
        fn upgrade(self) -> BoxedStream {
            self.conn
        }
    }
    impl<W> Responder for SequentialWriter<W>
    where
        W: Write + Send + Sync,
    {
        fn write_bytes(self, bytes: Vec<u8>) -> Result<(), std::io::Error> {
            self.send(&bytes)
        }
    }
}
pub mod framework {
    //! This module provides the application entry point.
    //!
    use std::{
        marker::PhantomData, net::{TcpListener, ToSocketAddrs},
        sync::Arc,
    };
    use crate::{
        connection::Connection, layers::{BoxLayer, DefaultResponseGroup, Layer},
        routing::Scope, tasks::TaskPool, type_cache::TypeCache, Request, Response,
    };
    #[cfg(not(feature = "tls"))]
    use crate::tasks::ConnectionTask;
    /// Main application entry point. Construct this type to run your application.
    pub struct App {
        tree: Scope,
        request_layer: BoxLayer<Request>,
        response_layer: BoxLayer<Response>,
        type_cache: TypeCache,
    }
    impl App {
        /// Constructs a new application
        pub fn builder(scope: impl Into<Scope>) -> Self {
            Self {
                tree: scope.into(),
                request_layer: Box::new(()),
                response_layer: Box::new(DefaultResponseGroup::new()),
                type_cache: TypeCache::new(),
            }
        }
        /// Overrides the default request `Layer` if one is set
        pub fn request_layer(
            mut self,
            layer: impl 'static + Layer<Request> + Send + Sync,
        ) -> Self {
            self.request_layer = Box::new(layer);
            self
        }
        /// Overrides the default response `Layer` if one is set
        pub fn response_layer(
            mut self,
            layer: impl 'static + Layer<Response> + Send + Sync,
        ) -> Self {
            self.response_layer = Box::new(layer);
            self
        }
        /// Sets the cache to be used by the application
        pub fn cache(mut self, cache: TypeCache) -> Self {
            self.type_cache = cache;
            self
        }
        /// Executes the application. This will currently never return.
        pub fn run<C>(self, address: impl ToSocketAddrs)
        where
            C: 'static + Connection,
        {
            let incoming = TcpListener::bind(address)
                .expect("Could not bind to local address");
            let type_cache = Arc::new(self.type_cache);
            let router = Arc::new(self.tree);
            let request_layer = Arc::new(self.request_layer);
            let response_layer = Arc::new(self.response_layer);
            let task_pool = TaskPool::new();
            loop {
                let Ok((stream, _addr)) = incoming.accept() else {
                    continue;
                };
                #[cfg(not(feature = "tls"))]
                let task = ConnectionTask::<C> {
                    task_pool: task_pool.clone(),
                    cache: type_cache.clone(),
                    stream,
                    router: router.clone(),
                    response_layer: response_layer.clone(),
                    request_layer: request_layer.clone(),
                    phantom_data: PhantomData,
                };
                task_pool.send_task(task);
            }
        }
    }
}
pub mod get_as_slice {
    pub trait GetAsSlice {
        fn get_as_slice(&self) -> &[u8];
    }
}
pub mod http_utils {
    //! This module provides http utility traits and functions for parsing and handling Requests and
    //! Responses
    use http::{Request, Response, Version};
    use std::io::{BufRead, Write};
    use crate::action::RawResponse;
    /// Errors while parsing requests.
    pub enum ParseError {
        MalformedRequest,
        ReadError,
        InvalidProtocolVer,
        InvalidRequestParts,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for ParseError {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    ParseError::MalformedRequest => "MalformedRequest",
                    ParseError::ReadError => "ReadError",
                    ParseError::InvalidProtocolVer => "InvalidProtocolVer",
                    ParseError::InvalidRequestParts => "InvalidRequestParts",
                },
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for ParseError {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for ParseError {
        #[inline]
        fn eq(&self, other: &ParseError) -> bool {
            let __self_tag = ::core::intrinsics::discriminant_value(self);
            let __arg1_tag = ::core::intrinsics::discriminant_value(other);
            __self_tag == __arg1_tag
        }
    }
    impl std::fmt::Display for ParseError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ParseError::MalformedRequest => {
                    f.write_fmt(format_args!("Malformed Request"))
                }
                ParseError::ReadError => f.write_fmt(format_args!("Read Error")),
                ParseError::InvalidProtocolVer => {
                    f.write_fmt(format_args!("Invalid Protocol"))
                }
                ParseError::InvalidRequestParts => {
                    f.write_fmt(format_args!("Invalid Request Parts"))
                }
            }
        }
    }
    impl std::error::Error for ParseError {}
    /// `Version` Extension trait
    pub trait VersionExt: Sized {
        /// Parse `Version` from a `&str`. Returns `Err` if the `&str` isn't a valid version of the HTTP protocol
        fn parse_version(s: &str) -> Result<Self, ParseError>;
        /// Convert a `Version` to a `String`
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
    /// Read a request from a source
    pub fn take_request<R>(reader: &mut R) -> Result<Request<()>, ParseError>
    where
        R: BufRead,
    {
        let mut lines = reader.lines();
        let line = lines
            .next()
            .ok_or(ParseError::MalformedRequest)?
            .map_err(|_| ParseError::ReadError)?;
        let mut parts = line.split(' ');
        let method = parts.next().ok_or(ParseError::MalformedRequest)?;
        let uri = parts.next().ok_or(ParseError::MalformedRequest)?;
        let version = parts.next().ok_or(ParseError::MalformedRequest)?;
        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .version(Version::parse_version(version)?);
        while let Some(line) = lines
            .next()
            .transpose()
            .map_err(|_| ParseError::ReadError)?
        {
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
    fn parse_response_line_into_buf<T>(
        buf: &mut Vec<u8>,
        request: &Response<T>,
    ) -> Result<(), std::io::Error> {
        buf.write_fmt(
            format_args!("{0} {1}\r\n", request.version().to_string(), request.status()),
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
        fn into_raw_response(self) -> RawResponse;
    }
    impl<T> ResponseExt for Response<T>
    where
        T: IntoRawBytes,
    {
        fn into_raw_response(self) -> RawResponse {
            self.map(IntoRawBytes::into_raw_bytes)
        }
    }
}
pub mod layers {
    use http::HeaderValue;
    use crate::{Request, Response};
    pub type BoxLayer<T> = Box<dyn 'static + Layer<T> + Send + Sync>;
    /// A collection of `Layers` to be executed in sequence.
    pub struct LayerGroup<I> {
        layers: Vec<BoxLayer<I>>,
    }
    impl<I> LayerGroup<I> {
        /// Constructs a new `LayerGroup`
        pub fn new() -> Self {
            LayerGroup { layers: Vec::new() }
        }
        /// pushes a new `Layer` to the stack
        pub fn add_layer(
            mut self,
            layer: impl 'static + Layer<I> + Send + Sync,
        ) -> Self {
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
        pub fn new() -> LayerGroup<Response> {
            let group = LayerGroup::new().add_layer(SetContentLength);
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
            let value = HeaderValue::from_str(
                    &{
                        let res = ::alloc::fmt::format(format_args!("{0}", bytes));
                        res
                    },
                )
                .expect("Failed to parse length as HeaderValue");
            data.headers_mut().insert("content-length", value);
        }
    }
    /// Sets the date header of all outgoing requests
    #[cfg(feature = "date")]
    pub struct SetDate;
    #[cfg(feature = "date")]
    impl Layer<Response> for SetDate {
        fn execute(&self, data: &mut Response) {
            let date = chrono::Utc::now().to_rfc2822();
            let value = HeaderValue::from_str(&date)
                .expect("Failed to convert date to header value");
            data.headers_mut().insert("date", value);
        }
    }
}
pub mod macros {}
pub mod resolve {
    use http::Method;
    use crate::{action::RawResponse, type_cache::TypeCacheKey, PathIter, RequestState};
    /// `Resolve` is a trait used to construct values needed to call a given `System`. All parameters
    /// of a `System` must implement `Resolve` to be valid.
    pub trait Resolve<'a>: Sized {
        type Output: 'a;
        fn resolve(
            ctx: &'a RequestState,
            path_iter: &mut PathIter,
        ) -> ResolveGuard<Self::Output>;
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
    impl<T> From<Option<T>> for ResolveGuard<T> {
        fn from(value: Option<T>) -> Self {
            match value {
                Some(v) => ResolveGuard::Value(v),
                None => ResolveGuard::None,
            }
        }
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
    impl<'a> Resolve<'a> for Get {
        type Output = Self;
        fn resolve(
            ctx: &'a RequestState,
            _path_iter: &mut PathIter,
        ) -> ResolveGuard<Self> {
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
    impl<'a> Resolve<'a> for Post {
        type Output = Self;
        fn resolve(
            ctx: &'a RequestState,
            _path_iter: &mut PathIter,
        ) -> ResolveGuard<Self> {
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
    impl<'a, K> Resolve<'a> for Query<K>
    where
        K: TypeCacheKey,
        K::Value: Clone,
    {
        type Output = Self;
        fn resolve(
            ctx: &'a RequestState,
            _path_iter: &mut PathIter,
        ) -> ResolveGuard<Self> {
            ctx.global_cache.get::<K>().map(|v| Query(v.clone())).into()
        }
    }
    /// A function with `Endpoint` as a parameter requires that the internal `path_iter` of the
    /// `RequestState` must be empty. This will only run if there are no trailing path parts of the
    /// uri.
    pub struct Endpoint;
    impl<'a> Resolve<'a> for Endpoint {
        type Output = Self;
        fn resolve(_ctx: &RequestState, path_iter: &mut PathIter) -> ResolveGuard<Self> {
            match path_iter.peek() {
                Some(v) if !v.is_empty() => ResolveGuard::None,
                _ => ResolveGuard::Value(Endpoint),
            }
        }
    }
    /// Collects the entire `Url` without modifying the `path_iter`
    pub struct Url(pub Vec<String>);
    impl<'a> Resolve<'a> for Url {
        type Output = Self;
        fn resolve(
            _ctx: &'a RequestState,
            path_iter: &mut PathIter,
        ) -> ResolveGuard<Self::Output> {
            ResolveGuard::Value(Url(path_iter.clone().map(|s| s.to_owned()).collect()))
        }
    }
    /// Consumes the next part of the url `path_iter`. Note that this will happen on call to its
    /// `resolve` method so ordering of parameters matter. Place any necessary guards before this
    /// method.
    pub struct UrlPart(pub String);
    impl<'a> Resolve<'a> for UrlPart {
        type Output = Self;
        fn resolve(
            _ctx: &'a RequestState,
            path_iter: &mut PathIter,
        ) -> ResolveGuard<Self> {
            path_iter.next().map(|i| UrlPart(i.to_string())).into()
        }
    }
    /// Collect the entire remaining url into a `Vec` Note that this will happen on call to its
    /// `resolve` method so ordering of parameters matter. Place any necessary guards before this
    /// method.
    pub struct UrlCollect(pub Vec<String>);
    impl<'a> Resolve<'a> for UrlCollect {
        type Output = Self;
        fn resolve(
            _ctx: &'a RequestState,
            path_iter: &mut PathIter,
        ) -> ResolveGuard<Self> {
            let mut collect = Vec::new();
            for part in path_iter.by_ref().map(|i| i.to_string()) {
                collect.push(part.to_string())
            }
            ResolveGuard::Value(UrlCollect(collect))
        }
    }
    impl<'a, 'b> Resolve<'a> for &'b [u8] {
        type Output = &'a [u8];
        fn resolve(
            ctx: &'a RequestState,
            _path_iter: &mut PathIter,
        ) -> ResolveGuard<Self::Output> {
            ResolveGuard::Value(ctx.request.body().get_as_slice())
        }
    }
    impl<'a, 'b> Resolve<'a> for &'b str {
        type Output = &'a str;
        fn resolve(
            ctx: &'a RequestState,
            _path_iter: &mut PathIter,
        ) -> ResolveGuard<Self::Output> {
            std::str::from_utf8(ctx.request.body().get_as_slice()).ok().into()
        }
    }
}
pub mod routing {
    use std::{borrow::BorrowMut, collections::HashMap};
    use http::Method;
    use crate::systems::{DynSystem, Handler, InsertHandler, IntoDynSystem, System};
    /// A Node in the Router tree.
    pub struct Scope {
        children: HashMap<String, Scope>,
        systems: Vec<DynSystem>,
    }
    impl Scope {
        /// Construct a new `Scope`
        pub fn new(systems: Vec<DynSystem>) -> Self {
            Self {
                children: HashMap::new(),
                systems,
            }
        }
        /// Construct an empty `Scope`
        pub fn empty() -> Self {
            Scope::new(::alloc::vec::Vec::new())
        }
        /// Add a `Scope` as a child of this node
        pub fn route(
            mut self,
            path: impl Into<String>,
            route: impl Into<Scope>,
        ) -> Self {
            self.children.insert(path.into(), route.into());
            self
        }
        /// Access the list of systems associated with this node
        pub fn systems(&self) -> &[DynSystem] {
            &self.systems
        }
        /// Scope to a child of this node by path
        pub fn get_child<'a>(&'a self, path: &str) -> Option<&'a Scope> {
            self.children.get(path)
        }
    }
    impl From<Vec<DynSystem>> for Scope {
        fn from(value: Vec<DynSystem>) -> Self {
            Scope::new(value)
        }
    }
    struct Node {
        handler: Option<Handler>,
        children: Vec<(Pattern, Node)>,
    }
    impl Node {
        fn new() -> Self {
            Self {
                handler: None,
                children: Vec::new(),
            }
        }
    }
    enum Pattern {
        Exact(String),
        Capture,
        Collect,
    }
    impl Pattern {
        fn new(s: &str) -> Self {
            if s.starts_with(":") {
                Pattern::Capture
            } else if s == "*" {
                Pattern::Collect
            } else {
                Pattern::Exact(s.to_string())
            }
        }
        fn exact(&self, rhs: &str) -> bool {
            match self {
                Pattern::Exact(s) => s == rhs,
                Pattern::Capture if rhs.starts_with(":") => true,
                Pattern::Collect if rhs == "*" => true,
                _ => false,
            }
        }
    }
    pub struct Router {
        root: Node,
    }
    impl Router {
        pub fn new() -> Self {
            Self { root: Node::new() }
        }
        pub fn add_route<T>(
            mut self,
            path: &str,
            handler: impl InsertHandler<T>,
        ) -> Self {
            let mut cursor = &mut self.root;
            for segment in path.split("/") {
                if segment.is_empty() {
                    continue;
                }
                if let Some(idx) = cursor
                    .children
                    .iter()
                    .position(|i| i.0.exact(segment))
                {
                    cursor = &mut cursor.children[idx].1;
                } else {
                    cursor.children.push((Pattern::new(segment), Node::new()));
                    {
                        ::std::io::_print(format_args!("{0}\n", cursor.children.len()));
                    };
                    cursor = cursor.children.last_mut().unwrap().1.borrow_mut();
                }
            }
            let mut new = Handler::new();
            handler.insert_to_handler(&mut new);
            cursor.handler = Some(new);
            self
        }
        pub fn route(&self, path: &str) -> Option<(&Handler, Vec<String>)> {
            let mut captured = ::alloc::vec::Vec::new();
            let mut cursor = &self.root;
            let mut iter = path.split("/");
            'outer: while let Some(segment) = iter.next() {
                if segment.is_empty() {
                    continue;
                }
                for (pattern, node) in cursor.children.iter() {
                    match pattern {
                        Pattern::Exact(s) if s == segment => {
                            cursor = &node;
                            break;
                        }
                        Pattern::Capture => {
                            captured.push(segment.to_string());
                            cursor = node;
                            break;
                        }
                        Pattern::Collect => {
                            captured.push(segment.to_string());
                            captured.extend(iter.map(String::from));
                            cursor = node;
                            break 'outer;
                        }
                        _ => {}
                    }
                }
            }
            cursor.handler.as_ref().map(|i| (i, captured))
        }
    }
}
pub mod systems {
    use std::collections::HashMap;
    use http::Method;
    use crate::{
        action::{Action, IntoAction},
        resolve::{Resolve, ResolveGuard},
        tasks::{PathIter, RequestState},
    };
    #[doc(hidden)]
    pub trait System<'a, T> {
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action;
    }
    #[doc(hidden)]
    pub struct DynSystem {
        inner: Box<
            dyn Fn(&RequestState, &mut PathIter) -> Action + 'static + Send + Sync,
        >,
    }
    impl DynSystem {
        pub fn new<A>(
            system: impl for<'a> System<'a, A> + 'static + Send + Sync + Copy,
        ) -> Self {
            DynSystem {
                inner: Box::new(move |ctx, path_iter| system.run(ctx, path_iter)),
            }
        }
        pub fn call(&self, ctx: &RequestState, path_iter: &mut PathIter) -> Action {
            (self.inner)(ctx, path_iter)
        }
    }
    #[doc(hidden)]
    pub trait IntoDynSystem<T> {
        fn into_dyn_system(self) -> DynSystem;
    }
    impl<T, A> IntoDynSystem<A> for T
    where
        T: for<'a> System<'a, A> + 'static + Send + Sync + Copy,
    {
        fn into_dyn_system(self) -> DynSystem {
            DynSystem::new(self)
        }
    }
    pub struct Handler {
        methods: HashMap<Method, DynSystem>,
    }
    impl Handler {
        pub(crate) fn new() -> Self {
            Self { methods: HashMap::new() }
        }
        pub(crate) fn insert(&mut self, method: Method, system: DynSystem) {
            self.methods.insert(method, system);
        }
    }
    pub trait InsertHandler<A> {
        fn insert_to_handler(self, handler: &mut Handler);
    }
    impl<
        'a,
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
        'a,
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
            ) -> RESPONSE
            + Fn(
                A::Output,
                B::Output,
                C::Output,
                D::Output,
                E::Output,
                F::Output,
                G::Output,
                H::Output,
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        A: Resolve<'a>,
        B: Resolve<'a>,
        C: Resolve<'a>,
        D: Resolve<'a>,
        E: Resolve<'a>,
        F: Resolve<'a>,
        G: Resolve<'a>,
        H: Resolve<'a>,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let A = match A::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let B = match B::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let C = match C::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let D = match D::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let E = match E::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let F = match F::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
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
            r.action()
        }
    }
    impl<
        'a,
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
        'a,
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
            ) -> RESPONSE
            + Fn(
                B::Output,
                C::Output,
                D::Output,
                E::Output,
                F::Output,
                G::Output,
                H::Output,
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        B: Resolve<'a>,
        C: Resolve<'a>,
        D: Resolve<'a>,
        E: Resolve<'a>,
        F: Resolve<'a>,
        G: Resolve<'a>,
        H: Resolve<'a>,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let B = match B::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let C = match C::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let D = match D::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let E = match E::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let F = match F::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
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
            r.action()
        }
    }
    impl<
        'a,
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
        'a,
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
            ) -> RESPONSE
            + Fn(
                C::Output,
                D::Output,
                E::Output,
                F::Output,
                G::Output,
                H::Output,
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        C: Resolve<'a>,
        D: Resolve<'a>,
        E: Resolve<'a>,
        F: Resolve<'a>,
        G: Resolve<'a>,
        H: Resolve<'a>,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let C = match C::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let D = match D::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let E = match E::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let F = match F::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
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
            r.action()
        }
    }
    impl<
        'a,
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
        'a,
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
            ) -> RESPONSE
            + Fn(
                D::Output,
                E::Output,
                F::Output,
                G::Output,
                H::Output,
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        D: Resolve<'a>,
        E: Resolve<'a>,
        F: Resolve<'a>,
        G: Resolve<'a>,
        H: Resolve<'a>,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let D = match D::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let E = match E::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let F = match F::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
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
            r.action()
        }
    }
    impl<
        'a,
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
        'a,
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
            ) -> RESPONSE
            + Fn(
                E::Output,
                F::Output,
                G::Output,
                H::Output,
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        E: Resolve<'a>,
        F: Resolve<'a>,
        G: Resolve<'a>,
        H: Resolve<'a>,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let E = match E::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let F = match F::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
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
            r.action()
        }
    }
    impl<
        'a,
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
    > System<
        'a,
        (RESPONSE, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z),
    > for BASE
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
            ) -> RESPONSE
            + Fn(
                F::Output,
                G::Output,
                H::Output,
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        F: Resolve<'a>,
        G: Resolve<'a>,
        H: Resolve<'a>,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let F = match F::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)>
    for BASE
    where
        BASE: Fn(G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                G::Output,
                H::Output,
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        G: Resolve<'a>,
        H: Resolve<'a>,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let G = match G::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)>
    for BASE
    where
        BASE: Fn(H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                H::Output,
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        H: Resolve<'a>,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let H = match H::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)>
    for BASE
    where
        BASE: Fn(I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let I = match I::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let J = match J::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let K = match K::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let L = match L::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let M = match M::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let N = match N::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let O = match O::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let P = match P::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let Q = match Q::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let R = match R::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let S = match S::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
        RESPONSE,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<'a, (RESPONSE, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let T = match T::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<'a, RESPONSE, U, V, W, X, Y, Z, BASE> System<'a, (RESPONSE, U, V, W, X, Y, Z)>
    for BASE
    where
        BASE: Fn(U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let U = match U::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<'a, RESPONSE, V, W, X, Y, Z, BASE> System<'a, (RESPONSE, V, W, X, Y, Z)>
    for BASE
    where
        BASE: Fn(V, W, X, Y, Z) -> RESPONSE
            + Fn(V::Output, W::Output, X::Output, Y::Output, Z::Output) -> RESPONSE,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let V = match V::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(V, W, X, Y, Z);
            r.action()
        }
    }
    impl<'a, RESPONSE, W, X, Y, Z, BASE> System<'a, (RESPONSE, W, X, Y, Z)> for BASE
    where
        BASE: Fn(W, X, Y, Z) -> RESPONSE
            + Fn(W::Output, X::Output, Y::Output, Z::Output) -> RESPONSE,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let W = match W::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(W, X, Y, Z);
            r.action()
        }
    }
    impl<'a, RESPONSE, X, Y, Z, BASE> System<'a, (RESPONSE, X, Y, Z)> for BASE
    where
        BASE: Fn(X, Y, Z) -> RESPONSE + Fn(X::Output, Y::Output, Z::Output) -> RESPONSE,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let X = match X::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(X, Y, Z);
            r.action()
        }
    }
    impl<'a, RESPONSE, Y, Z, BASE> System<'a, (RESPONSE, Y, Z)> for BASE
    where
        BASE: Fn(Y, Z) -> RESPONSE + Fn(Y::Output, Z::Output) -> RESPONSE,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let Y = match Y::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(Y, Z);
            r.action()
        }
    }
    impl<'a, RESPONSE, Z, BASE> System<'a, (RESPONSE, Z)> for BASE
    where
        BASE: Fn(Z) -> RESPONSE + Fn(Z::Output) -> RESPONSE,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            #[allow(non_snake_case)]
            let Z = match Z::resolve(ctx, path_iter) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(Z);
            r.action()
        }
    }
    impl<'a, RESPONSE, BASE> System<'a, (RESPONSE,)> for BASE
    where
        BASE: Fn() -> RESPONSE + Fn() -> RESPONSE,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Action {
            let r = self();
            r.action()
        }
    }
    impl<
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
    > InsertHandler<(B, D, F, H, J, L, N, P, R, T, V, X, Z)>
    for (
        (Method, A),
        (Method, C),
        (Method, E),
        (Method, G),
        (Method, I),
        (Method, K),
        (Method, M),
        (Method, O),
        (Method, Q),
        (Method, S),
        (Method, U),
        (Method, W),
        (Method, Y),
    )
    where
        A: IntoDynSystem<B>,
        C: IntoDynSystem<D>,
        E: IntoDynSystem<F>,
        G: IntoDynSystem<H>,
        I: IntoDynSystem<J>,
        K: IntoDynSystem<L>,
        M: IntoDynSystem<N>,
        O: IntoDynSystem<P>,
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (A, C, E, G, I, K, M, O, Q, S, U, W, Y) = self;
            handler.insert(A.0, A.1.into_dyn_system());
            handler.insert(C.0, C.1.into_dyn_system());
            handler.insert(E.0, E.1.into_dyn_system());
            handler.insert(G.0, G.1.into_dyn_system());
            handler.insert(I.0, I.1.into_dyn_system());
            handler.insert(K.0, K.1.into_dyn_system());
            handler.insert(M.0, M.1.into_dyn_system());
            handler.insert(O.0, O.1.into_dyn_system());
            handler.insert(Q.0, Q.1.into_dyn_system());
            handler.insert(S.0, S.1.into_dyn_system());
            handler.insert(U.0, U.1.into_dyn_system());
            handler.insert(W.0, W.1.into_dyn_system());
            handler.insert(Y.0, Y.1.into_dyn_system());
        }
    }
    impl<
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
    > InsertHandler<(D, F, H, J, L, N, P, R, T, V, X, Z)>
    for (
        (Method, C),
        (Method, E),
        (Method, G),
        (Method, I),
        (Method, K),
        (Method, M),
        (Method, O),
        (Method, Q),
        (Method, S),
        (Method, U),
        (Method, W),
        (Method, Y),
    )
    where
        C: IntoDynSystem<D>,
        E: IntoDynSystem<F>,
        G: IntoDynSystem<H>,
        I: IntoDynSystem<J>,
        K: IntoDynSystem<L>,
        M: IntoDynSystem<N>,
        O: IntoDynSystem<P>,
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (C, E, G, I, K, M, O, Q, S, U, W, Y) = self;
            handler.insert(C.0, C.1.into_dyn_system());
            handler.insert(E.0, E.1.into_dyn_system());
            handler.insert(G.0, G.1.into_dyn_system());
            handler.insert(I.0, I.1.into_dyn_system());
            handler.insert(K.0, K.1.into_dyn_system());
            handler.insert(M.0, M.1.into_dyn_system());
            handler.insert(O.0, O.1.into_dyn_system());
            handler.insert(Q.0, Q.1.into_dyn_system());
            handler.insert(S.0, S.1.into_dyn_system());
            handler.insert(U.0, U.1.into_dyn_system());
            handler.insert(W.0, W.1.into_dyn_system());
            handler.insert(Y.0, Y.1.into_dyn_system());
        }
    }
    impl<
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
    > InsertHandler<(F, H, J, L, N, P, R, T, V, X, Z)>
    for (
        (Method, E),
        (Method, G),
        (Method, I),
        (Method, K),
        (Method, M),
        (Method, O),
        (Method, Q),
        (Method, S),
        (Method, U),
        (Method, W),
        (Method, Y),
    )
    where
        E: IntoDynSystem<F>,
        G: IntoDynSystem<H>,
        I: IntoDynSystem<J>,
        K: IntoDynSystem<L>,
        M: IntoDynSystem<N>,
        O: IntoDynSystem<P>,
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (E, G, I, K, M, O, Q, S, U, W, Y) = self;
            handler.insert(E.0, E.1.into_dyn_system());
            handler.insert(G.0, G.1.into_dyn_system());
            handler.insert(I.0, I.1.into_dyn_system());
            handler.insert(K.0, K.1.into_dyn_system());
            handler.insert(M.0, M.1.into_dyn_system());
            handler.insert(O.0, O.1.into_dyn_system());
            handler.insert(Q.0, Q.1.into_dyn_system());
            handler.insert(S.0, S.1.into_dyn_system());
            handler.insert(U.0, U.1.into_dyn_system());
            handler.insert(W.0, W.1.into_dyn_system());
            handler.insert(Y.0, Y.1.into_dyn_system());
        }
    }
    impl<
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
    > InsertHandler<(H, J, L, N, P, R, T, V, X, Z)>
    for (
        (Method, G),
        (Method, I),
        (Method, K),
        (Method, M),
        (Method, O),
        (Method, Q),
        (Method, S),
        (Method, U),
        (Method, W),
        (Method, Y),
    )
    where
        G: IntoDynSystem<H>,
        I: IntoDynSystem<J>,
        K: IntoDynSystem<L>,
        M: IntoDynSystem<N>,
        O: IntoDynSystem<P>,
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (G, I, K, M, O, Q, S, U, W, Y) = self;
            handler.insert(G.0, G.1.into_dyn_system());
            handler.insert(I.0, I.1.into_dyn_system());
            handler.insert(K.0, K.1.into_dyn_system());
            handler.insert(M.0, M.1.into_dyn_system());
            handler.insert(O.0, O.1.into_dyn_system());
            handler.insert(Q.0, Q.1.into_dyn_system());
            handler.insert(S.0, S.1.into_dyn_system());
            handler.insert(U.0, U.1.into_dyn_system());
            handler.insert(W.0, W.1.into_dyn_system());
            handler.insert(Y.0, Y.1.into_dyn_system());
        }
    }
    impl<
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
    > InsertHandler<(J, L, N, P, R, T, V, X, Z)>
    for (
        (Method, I),
        (Method, K),
        (Method, M),
        (Method, O),
        (Method, Q),
        (Method, S),
        (Method, U),
        (Method, W),
        (Method, Y),
    )
    where
        I: IntoDynSystem<J>,
        K: IntoDynSystem<L>,
        M: IntoDynSystem<N>,
        O: IntoDynSystem<P>,
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (I, K, M, O, Q, S, U, W, Y) = self;
            handler.insert(I.0, I.1.into_dyn_system());
            handler.insert(K.0, K.1.into_dyn_system());
            handler.insert(M.0, M.1.into_dyn_system());
            handler.insert(O.0, O.1.into_dyn_system());
            handler.insert(Q.0, Q.1.into_dyn_system());
            handler.insert(S.0, S.1.into_dyn_system());
            handler.insert(U.0, U.1.into_dyn_system());
            handler.insert(W.0, W.1.into_dyn_system());
            handler.insert(Y.0, Y.1.into_dyn_system());
        }
    }
    impl<
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
    > InsertHandler<(L, N, P, R, T, V, X, Z)>
    for (
        (Method, K),
        (Method, M),
        (Method, O),
        (Method, Q),
        (Method, S),
        (Method, U),
        (Method, W),
        (Method, Y),
    )
    where
        K: IntoDynSystem<L>,
        M: IntoDynSystem<N>,
        O: IntoDynSystem<P>,
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (K, M, O, Q, S, U, W, Y) = self;
            handler.insert(K.0, K.1.into_dyn_system());
            handler.insert(M.0, M.1.into_dyn_system());
            handler.insert(O.0, O.1.into_dyn_system());
            handler.insert(Q.0, Q.1.into_dyn_system());
            handler.insert(S.0, S.1.into_dyn_system());
            handler.insert(U.0, U.1.into_dyn_system());
            handler.insert(W.0, W.1.into_dyn_system());
            handler.insert(Y.0, Y.1.into_dyn_system());
        }
    }
    impl<M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z> InsertHandler<(N, P, R, T, V, X, Z)>
    for (
        (Method, M),
        (Method, O),
        (Method, Q),
        (Method, S),
        (Method, U),
        (Method, W),
        (Method, Y),
    )
    where
        M: IntoDynSystem<N>,
        O: IntoDynSystem<P>,
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (M, O, Q, S, U, W, Y) = self;
            handler.insert(M.0, M.1.into_dyn_system());
            handler.insert(O.0, O.1.into_dyn_system());
            handler.insert(Q.0, Q.1.into_dyn_system());
            handler.insert(S.0, S.1.into_dyn_system());
            handler.insert(U.0, U.1.into_dyn_system());
            handler.insert(W.0, W.1.into_dyn_system());
            handler.insert(Y.0, Y.1.into_dyn_system());
        }
    }
    impl<O, P, Q, R, S, T, U, V, W, X, Y, Z> InsertHandler<(P, R, T, V, X, Z)>
    for ((Method, O), (Method, Q), (Method, S), (Method, U), (Method, W), (Method, Y))
    where
        O: IntoDynSystem<P>,
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (O, Q, S, U, W, Y) = self;
            handler.insert(O.0, O.1.into_dyn_system());
            handler.insert(Q.0, Q.1.into_dyn_system());
            handler.insert(S.0, S.1.into_dyn_system());
            handler.insert(U.0, U.1.into_dyn_system());
            handler.insert(W.0, W.1.into_dyn_system());
            handler.insert(Y.0, Y.1.into_dyn_system());
        }
    }
    impl<Q, R, S, T, U, V, W, X, Y, Z> InsertHandler<(R, T, V, X, Z)>
    for ((Method, Q), (Method, S), (Method, U), (Method, W), (Method, Y))
    where
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (Q, S, U, W, Y) = self;
            handler.insert(Q.0, Q.1.into_dyn_system());
            handler.insert(S.0, S.1.into_dyn_system());
            handler.insert(U.0, U.1.into_dyn_system());
            handler.insert(W.0, W.1.into_dyn_system());
            handler.insert(Y.0, Y.1.into_dyn_system());
        }
    }
    impl<S, T, U, V, W, X, Y, Z> InsertHandler<(T, V, X, Z)>
    for ((Method, S), (Method, U), (Method, W), (Method, Y))
    where
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (S, U, W, Y) = self;
            handler.insert(S.0, S.1.into_dyn_system());
            handler.insert(U.0, U.1.into_dyn_system());
            handler.insert(W.0, W.1.into_dyn_system());
            handler.insert(Y.0, Y.1.into_dyn_system());
        }
    }
    impl<U, V, W, X, Y, Z> InsertHandler<(V, X, Z)>
    for ((Method, U), (Method, W), (Method, Y))
    where
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (U, W, Y) = self;
            handler.insert(U.0, U.1.into_dyn_system());
            handler.insert(W.0, W.1.into_dyn_system());
            handler.insert(Y.0, Y.1.into_dyn_system());
        }
    }
    impl<W, X, Y, Z> InsertHandler<(X, Z)> for ((Method, W), (Method, Y))
    where
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (W, Y) = self;
            handler.insert(W.0, W.1.into_dyn_system());
            handler.insert(Y.0, Y.1.into_dyn_system());
        }
    }
    impl<Y, Z> InsertHandler<(Z,)> for ((Method, Y),)
    where
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (Y,) = self;
            handler.insert(Y.0, Y.1.into_dyn_system());
        }
    }
}
pub mod type_cache {
    use std::{
        any::{Any, TypeId},
        collections::HashMap,
    };
    type Value = Box<dyn Any + Sync + Send>;
    /// This trait allows a type to be used as a key in a `TypeCache`
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
pub use action::{Action, IntoResponse};
pub use connection::Http1;
pub use framework::App;
pub use http_utils::IntoRawBytes;
pub use layers::{DefaultResponseGroup, Layer};
pub use resolve::{Resolve, ResolveGuard};
pub use routing::Scope;
pub use tasks::{PathIter, RequestState};
pub use type_cache::{TypeCache, TypeCacheKey};
pub use http;
/// Request type used by most of `foxhole`
pub type Request = tasks::BoxedBodyRequest;
/// Response type used by most of `foxhole`
pub type Response = action::RawResponse;
#![feature(prelude_import)]
/*!<div align="center">
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
*/
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
mod lazy {
    use std::{cell::OnceCell, sync::mpsc::{channel, Receiver, Sender}};
    use crate::get_as_slice::GetAsSlice;
    pub struct Lazy<T> {
        receiver: Receiver<T>,
        value: OnceCell<T>,
    }
    impl Lazy<Vec<u8>> {
        /// Constructs a new instance of `Lazy` and returns it's corresponding `Sender`
        pub fn new() -> (Self, Sender<Vec<u8>>) {
            let (sender, receiver) = channel();
            (
                Lazy {
                    receiver,
                    value: OnceCell::new(),
                },
                sender,
            )
        }
        /// This call blocks until the body has been read from the `TcpStream`
        pub fn get(&self) -> &[u8] {
            self.value.get_or_init(|| self.receiver.recv().unwrap())
        }
    }
    impl GetAsSlice for Lazy<Vec<u8>> {
        fn get_as_slice(&self) -> &[u8] {
            self.get()
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
        W: Write,
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
mod tasks {
    use std::{
        collections::VecDeque, iter::Peekable, marker::PhantomData, net::TcpStream,
        str::Split,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Condvar, Mutex,
        },
        time::Duration,
    };
    use http::Request;
    use crate::{
        connection::{Connection, Responder},
        get_as_slice::GetAsSlice, layers::BoxLayer, type_cache::TypeCache, Action,
        IntoResponse, Response, Router,
    };
    const MIN_THREADS: usize = 4;
    const TIMEOUT: u64 = 5;
    /// Request with a boxed body implementing `GetAsSlice` This is the standard request type
    /// throughout the library
    pub type BoxedBodyRequest = Request<Box<dyn 'static + GetAsSlice + Send>>;
    /// The url iterator type used by the library. This can be found in `RequestState` accessed via the
    /// `Resolve` trait
    pub type PathIter<'a> = Peekable<Split<'a, &'static str>>;
    pub(crate) trait Task {
        fn run(self: Box<Self>);
    }
    pub(crate) struct ConnectionTask<C> {
        pub task_pool: TaskPool,
        /// An application global type cache
        pub cache: Arc<TypeCache>,
        pub stream: TcpStream,
        /// A handle to the applications router tree
        pub router: Arc<Router>,
        pub request_layer: Arc<BoxLayer<crate::Request>>,
        pub response_layer: Arc<BoxLayer<Response>>,
        pub phantom_data: PhantomData<C>,
    }
    impl<C> Task for ConnectionTask<C>
    where
        C: 'static + Connection,
    {
        fn run(self: Box<Self>) {
            let Ok(mut connection) = C::new(Box::new(self.stream)) else {
                return;
            };
            connection
                .set_timeout(Some(Duration::from_secs(TIMEOUT)))
                .expect("Shouldn't fail unless duration is 0");
            while let Ok((request, responder)) = connection.next_frame() {
                let r = request
                    .map(|i| {
                        let b: Box<dyn 'static + GetAsSlice + Send> = Box::new(i);
                        b
                    });
                let mut should_close: bool = true;
                if let Some(header) = r
                    .headers()
                    .get("connection")
                    .and_then(|i| i.to_str().ok())
                {
                    should_close = header != "keep-alive";
                }
                self.task_pool
                    .send_task(RequestTask::<_, C> {
                        cache: self.cache.clone(),
                        upgrade: None,
                        request: r,
                        responder,
                        router: self.router.clone(),
                        request_layer: self.request_layer.clone(),
                        response_layer: self.response_layer.clone(),
                    });
                if should_close {
                    break;
                }
            }
        }
    }
    pub(crate) struct RequestTask<R, C> {
        pub cache: Arc<TypeCache>,
        pub request: BoxedBodyRequest,
        pub responder: R,
        /// A handle to the applications router tree
        pub router: Arc<Router>,
        /// This field is only used on `websocket`
        #[allow(dead_code)]
        pub upgrade: Option<C>,
        pub request_layer: Arc<BoxLayer<crate::Request>>,
        pub response_layer: Arc<BoxLayer<Response>>,
    }
    impl<R, C> Task for RequestTask<R, C>
    where
        R: Responder,
        C: Connection,
    {
        fn run(mut self: Box<Self>) {
            self.request_layer.execute(&mut self.request);
            let uri = self.request.uri().to_string();
            let Some((handler, captures)) = self.router.route(&uri) else {
                return;
            };
            let Some(system) = handler.get(self.request.method()) else {
                return;
            };
            let ctx = RequestState {
                global_cache: self.cache.clone(),
                request: self.request,
            };
            let action = system.call(&ctx, captures);
            match action {
                Action::Respond(mut r) => {
                    self.response_layer.execute(&mut r);
                    let _ = self.responder.respond(r);
                    return;
                }
                Action::None => {}
            }
        }
    }
    /// Holds the state of the request handling. This can be accessed via the `Resolve` trait.
    pub struct RequestState {
        pub global_cache: Arc<TypeCache>,
        pub request: BoxedBodyRequest,
    }
    struct Shared {
        /// Pool of tasks that need to be run
        pool: Mutex<VecDeque<Box<dyn Task + Send + 'static>>>,
        /// Conditional var used to sleep and wake threads
        condvar: Condvar,
        /// Total number of threads currently waiting for a task
        waiting_tasks: AtomicUsize,
    }
    impl Shared {
        fn waiting(&self) {
            self.waiting_tasks.fetch_add(1, Ordering::Release);
        }
        fn release(&self) {
            self.waiting_tasks.fetch_sub(1, Ordering::Release);
        }
    }
    pub(crate) struct TaskPool {
        shared: Arc<Shared>,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for TaskPool {
        #[inline]
        fn clone(&self) -> TaskPool {
            TaskPool {
                shared: ::core::clone::Clone::clone(&self.shared),
            }
        }
    }
    impl TaskPool {
        pub fn new() -> Self {
            let pool = TaskPool {
                shared: Arc::new(Shared {
                    pool: Mutex::new(VecDeque::new()),
                    condvar: Condvar::new(),
                    waiting_tasks: AtomicUsize::new(0),
                }),
            };
            for _ in 0..MIN_THREADS {
                pool.spawn_thread(false, None);
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
        fn spawn_thread(
            &self,
            should_cull: bool,
            initial_task: Option<Box<dyn Task + Send>>,
        ) {
            let shared = self.shared.clone();
            std::thread::spawn(move || {
                match initial_task {
                    Some(task) => task.run(),
                    None => {}
                }
                loop {
                    let mut pool = shared.pool.lock().unwrap();
                    shared.waiting();
                    if should_cull {
                        let (new, timeout) = shared
                            .condvar
                            .wait_timeout(pool, Duration::from_secs(5))
                            .unwrap();
                        if timeout.timed_out() {
                            break;
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
                    task.run();
                }
                shared.release()
            });
        }
        /// Adds a task to the task pool and spawns a thread if there is none available
        ///
        /// # Panics
        /// This function can panic if the mutex is poisoned. Mutex poisoning will likely remain
        /// unhandled in the foreseeable future until a graceful shutdown mechanism is provided.
        pub fn send_task<T>(&self, task: T)
        where
            T: Task + Send + 'static,
        {
            if self.shared.waiting_tasks.load(Ordering::Acquire) < MIN_THREADS {
                self.spawn_thread(true, Some(Box::new(task)));
            } else {
                self.shared.pool.lock().unwrap().push_back(Box::new(task));
                self.shared.condvar.notify_one();
            }
        }
    }
}
mod tls_connection {
    use std::{
        io::{Read, Write},
        net::TcpStream, sync::{Arc, RwLock},
    };
    use rustls::ServerConnection;
    pub struct TlsConnection {
        pub stream: TcpStream,
        pub conn: Arc<RwLock<ServerConnection>>,
    }
    impl TlsConnection {
        pub fn new(stream: TcpStream, conn: Arc<RwLock<ServerConnection>>) -> Self {
            Self { stream, conn }
        }
    }
    impl Read for TlsConnection {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.conn.write().unwrap().reader().read(buf)
        }
    }
    impl Write for TlsConnection {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.conn.write().unwrap().writer().write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.conn.write().unwrap().complete_io(&mut self.stream).map(|_| ())
        }
    }
}
pub mod handler {
    use std::collections::HashMap;
    use crate::systems::{DynSystem, IntoDynSystem};
    pub enum Method<T> {
        Get(T),
        Post(T),
    }
    pub struct Handler {
        methods: HashMap<http::Method, DynSystem>,
    }
    impl Handler {
        pub(crate) fn new() -> Self {
            Self { methods: HashMap::new() }
        }
        pub(crate) fn insert(&mut self, method: http::Method, system: DynSystem) {
            self.methods.insert(method, system);
        }
        pub(crate) fn get(&self, method: &http::Method) -> Option<&DynSystem> {
            self.methods.get(method)
        }
    }
    pub trait InsertHandler<A> {
        fn insert_to_handler(self, handler: &mut Handler);
    }
    #[allow(unused_parens)]
    impl<
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
    > InsertHandler<(B, D, F, H, J, L, N, P, R, T, V, X, Z)>
    for (
        Method<A>,
        Method<C>,
        Method<E>,
        Method<G>,
        Method<I>,
        Method<K>,
        Method<M>,
        Method<O>,
        Method<Q>,
        Method<S>,
        Method<U>,
        Method<W>,
        Method<Y>,
    )
    where
        A: IntoDynSystem<B>,
        C: IntoDynSystem<D>,
        E: IntoDynSystem<F>,
        G: IntoDynSystem<H>,
        I: IntoDynSystem<J>,
        K: IntoDynSystem<L>,
        M: IntoDynSystem<N>,
        O: IntoDynSystem<P>,
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (A, C, E, G, I, K, M, O, Q, S, U, W, Y) = self;
            match A {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match C {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match E {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match G {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match I {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match K {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match M {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match O {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Q {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match S {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match U {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match W {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Y {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
        }
    }
    #[allow(unused_parens)]
    impl<
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
    > InsertHandler<(D, F, H, J, L, N, P, R, T, V, X, Z)>
    for (
        Method<C>,
        Method<E>,
        Method<G>,
        Method<I>,
        Method<K>,
        Method<M>,
        Method<O>,
        Method<Q>,
        Method<S>,
        Method<U>,
        Method<W>,
        Method<Y>,
    )
    where
        C: IntoDynSystem<D>,
        E: IntoDynSystem<F>,
        G: IntoDynSystem<H>,
        I: IntoDynSystem<J>,
        K: IntoDynSystem<L>,
        M: IntoDynSystem<N>,
        O: IntoDynSystem<P>,
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (C, E, G, I, K, M, O, Q, S, U, W, Y) = self;
            match C {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match E {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match G {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match I {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match K {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match M {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match O {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Q {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match S {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match U {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match W {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Y {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
        }
    }
    #[allow(unused_parens)]
    impl<
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
    > InsertHandler<(F, H, J, L, N, P, R, T, V, X, Z)>
    for (
        Method<E>,
        Method<G>,
        Method<I>,
        Method<K>,
        Method<M>,
        Method<O>,
        Method<Q>,
        Method<S>,
        Method<U>,
        Method<W>,
        Method<Y>,
    )
    where
        E: IntoDynSystem<F>,
        G: IntoDynSystem<H>,
        I: IntoDynSystem<J>,
        K: IntoDynSystem<L>,
        M: IntoDynSystem<N>,
        O: IntoDynSystem<P>,
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (E, G, I, K, M, O, Q, S, U, W, Y) = self;
            match E {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match G {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match I {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match K {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match M {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match O {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Q {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match S {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match U {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match W {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Y {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
        }
    }
    #[allow(unused_parens)]
    impl<
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
    > InsertHandler<(H, J, L, N, P, R, T, V, X, Z)>
    for (
        Method<G>,
        Method<I>,
        Method<K>,
        Method<M>,
        Method<O>,
        Method<Q>,
        Method<S>,
        Method<U>,
        Method<W>,
        Method<Y>,
    )
    where
        G: IntoDynSystem<H>,
        I: IntoDynSystem<J>,
        K: IntoDynSystem<L>,
        M: IntoDynSystem<N>,
        O: IntoDynSystem<P>,
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (G, I, K, M, O, Q, S, U, W, Y) = self;
            match G {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match I {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match K {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match M {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match O {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Q {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match S {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match U {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match W {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Y {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
        }
    }
    #[allow(unused_parens)]
    impl<
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
    > InsertHandler<(J, L, N, P, R, T, V, X, Z)>
    for (
        Method<I>,
        Method<K>,
        Method<M>,
        Method<O>,
        Method<Q>,
        Method<S>,
        Method<U>,
        Method<W>,
        Method<Y>,
    )
    where
        I: IntoDynSystem<J>,
        K: IntoDynSystem<L>,
        M: IntoDynSystem<N>,
        O: IntoDynSystem<P>,
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (I, K, M, O, Q, S, U, W, Y) = self;
            match I {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match K {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match M {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match O {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Q {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match S {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match U {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match W {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Y {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
        }
    }
    #[allow(unused_parens)]
    impl<
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
    > InsertHandler<(L, N, P, R, T, V, X, Z)>
    for (
        Method<K>,
        Method<M>,
        Method<O>,
        Method<Q>,
        Method<S>,
        Method<U>,
        Method<W>,
        Method<Y>,
    )
    where
        K: IntoDynSystem<L>,
        M: IntoDynSystem<N>,
        O: IntoDynSystem<P>,
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (K, M, O, Q, S, U, W, Y) = self;
            match K {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match M {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match O {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Q {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match S {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match U {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match W {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Y {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
        }
    }
    #[allow(unused_parens)]
    impl<M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z> InsertHandler<(N, P, R, T, V, X, Z)>
    for (Method<M>, Method<O>, Method<Q>, Method<S>, Method<U>, Method<W>, Method<Y>)
    where
        M: IntoDynSystem<N>,
        O: IntoDynSystem<P>,
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (M, O, Q, S, U, W, Y) = self;
            match M {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match O {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Q {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match S {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match U {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match W {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Y {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
        }
    }
    #[allow(unused_parens)]
    impl<O, P, Q, R, S, T, U, V, W, X, Y, Z> InsertHandler<(P, R, T, V, X, Z)>
    for (Method<O>, Method<Q>, Method<S>, Method<U>, Method<W>, Method<Y>)
    where
        O: IntoDynSystem<P>,
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (O, Q, S, U, W, Y) = self;
            match O {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Q {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match S {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match U {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match W {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Y {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
        }
    }
    #[allow(unused_parens)]
    impl<Q, R, S, T, U, V, W, X, Y, Z> InsertHandler<(R, T, V, X, Z)>
    for (Method<Q>, Method<S>, Method<U>, Method<W>, Method<Y>)
    where
        Q: IntoDynSystem<R>,
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (Q, S, U, W, Y) = self;
            match Q {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match S {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match U {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match W {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Y {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
        }
    }
    #[allow(unused_parens)]
    impl<S, T, U, V, W, X, Y, Z> InsertHandler<(T, V, X, Z)>
    for (Method<S>, Method<U>, Method<W>, Method<Y>)
    where
        S: IntoDynSystem<T>,
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (S, U, W, Y) = self;
            match S {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match U {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match W {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Y {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
        }
    }
    #[allow(unused_parens)]
    impl<U, V, W, X, Y, Z> InsertHandler<(V, X, Z)> for (Method<U>, Method<W>, Method<Y>)
    where
        U: IntoDynSystem<V>,
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (U, W, Y) = self;
            match U {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match W {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Y {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
        }
    }
    #[allow(unused_parens)]
    impl<W, X, Y, Z> InsertHandler<(X, Z)> for (Method<W>, Method<Y>)
    where
        W: IntoDynSystem<X>,
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (W, Y) = self;
            match W {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
            match Y {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
        }
    }
    #[allow(unused_parens)]
    impl<Y, Z> InsertHandler<(Z)> for (Method<Y>)
    where
        Y: IntoDynSystem<Z>,
    {
        fn insert_to_handler(self, handler: &mut Handler) {
            #[allow(non_snake_case)]
            let (Y) = self;
            match Y {
                Method::Get(v) => handler.insert(http::Method::GET, v.into_dyn_system()),
                _ => {}
            }
        }
    }
}
pub mod action {
    use http::{Response, Version};
    use crate::http_utils::IntoRawBytes;
    pub type RawResponse = Response<Vec<u8>>;
    /// This is a helper trait to remove the unnecessary `Option` many response types don't need as
    /// they do not affect control flow.
    pub trait IntoResponse {
        fn response(self) -> RawResponse;
    }
    pub enum Action {
        Respond(RawResponse),
        None,
    }
    /// All `System`s must return a type implementing `Action`. This trait decides the
    /// behaviour of the underlying router.
    /// - `Action::None` The router will continue to the next system
    /// - `Action::Respond` The router will respond immediately. No subsequent systems will be run
    /// - `Action::Handle` The task will transfer ownership of the stream to the fn. No subsequent
    /// systems will be run
    pub trait IntoAction {
        fn action(self) -> Action;
    }
    impl<T> IntoAction for T
    where
        T: IntoResponse,
    {
        fn action(self) -> Action {
            Action::Respond(self.response())
        }
    }
    impl<T> IntoAction for Response<T>
    where
        T: IntoRawBytes,
    {
        fn action(self) -> Action {
            Action::Respond(self.map(IntoRawBytes::into_raw_bytes))
        }
    }
    impl IntoAction for () {
        fn action(self) -> Action {
            Action::None
        }
    }
    impl<T> IntoAction for Option<T>
    where
        T: IntoAction,
    {
        fn action(self) -> Action {
            match self {
                Some(v) => v.action(),
                None => Action::None,
            }
        }
    }
    impl<T, E> IntoAction for Result<T, E>
    where
        T: IntoAction,
        E: IntoAction,
    {
        fn action(self) -> Action {
            match self {
                Ok(v) => v.action(),
                Err(e) => e.action(),
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
    /// Creates a response with the content-type `application/x-binary` there may be a better MIME type
    /// to use for this.
    pub struct Raw(Vec<u8>);
    impl IntoResponse for Raw {
        fn response(self) -> RawResponse {
            Response::builder()
                .version(Version::HTTP_11)
                .status(200)
                .header("Content-Type", "application/x-binary")
                .header(
                    "Content-Length",
                    {
                        let res = ::alloc::fmt::format(
                            format_args!("{0}", self.0.len()),
                        );
                        res
                    },
                )
                .body(self.0)
                .unwrap()
        }
    }
    /// Creates a response with the content-type `text/plain`
    pub struct Plain(pub String);
    impl IntoResponse for Plain {
        fn response(self) -> RawResponse {
            let bytes = self.0.into_bytes();
            Response::builder()
                .version(Version::HTTP_11)
                .status(200)
                .header("Content-Type", "text/plain; charset=utf-8")
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
    /// Creates a response with the content-type `text/html`
    pub struct Html(pub String);
    impl IntoResponse for Html {
        fn response(self) -> RawResponse {
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
    /// Creates a response with the content-type `text/css`
    pub struct Css(pub String);
    impl IntoResponse for Css {
        fn response(self) -> RawResponse {
            let bytes = self.0.into_bytes();
            Response::builder()
                .version(Version::HTTP_11)
                .status(200)
                .header("Content-Type", "text/css; charset=utf-8")
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
    /// Creates a response with the content-type `text/javascript`
    pub struct Js(pub String);
    impl IntoResponse for Js {
        fn response(self) -> RawResponse {
            let bytes = self.0.into_bytes();
            Response::builder()
                .version(Version::HTTP_11)
                .status(200)
                .header("Content-Type", "text/javascript; charset=utf-8")
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
}
pub mod connection {
    use std::{
        io::{BufReader, ErrorKind, Read, Write},
        net::TcpStream, sync::mpsc::{Receiver, Sender},
        time::Duration,
    };
    use http::Request;
    use crate::{
        action::RawResponse, get_as_slice::GetAsSlice,
        http_utils::{take_request, IntoRawBytes},
        lazy::Lazy, sequential_writer::{self, SequentialWriter},
        tls_connection::TlsConnection,
    };
    /// A marker used to encapsulate the required traits for a stream used by `Http1`
    pub trait BoxedStreamMarker: Read + Write + BoxedTryClone + SetTimeout + Send + Sync {}
    impl<T> BoxedStreamMarker for T
    where
        T: Read + Write + BoxedTryClone + SetTimeout + Send + Sync,
    {}
    pub type BoxedStream = Box<dyn BoxedStreamMarker>;
    /// A trait providing a `set_timeout` function for streams
    pub trait SetTimeout {
        /// Set the timeout for reads and writes on the current object
        fn set_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()>;
    }
    impl SetTimeout for TcpStream {
        fn set_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()> {
            self.set_read_timeout(timeout)?;
            self.set_write_timeout(timeout)
        }
    }
    impl SetTimeout for TlsConnection {
        fn set_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()> {
            self.stream.set_read_timeout(timeout)?;
            self.stream.set_write_timeout(timeout)
        }
    }
    /// A trait providing a method of cloning for boxed streams
    pub trait BoxedTryClone {
        fn try_clone(&self) -> std::io::Result<BoxedStream>;
    }
    impl BoxedTryClone for TcpStream {
        fn try_clone(&self) -> std::io::Result<BoxedStream> {
            self.try_clone().map(|s| Box::new(s) as BoxedStream)
        }
    }
    impl BoxedTryClone for TlsConnection {
        fn try_clone(&self) -> std::io::Result<BoxedStream> {
            self.stream
                .try_clone()
                .map(|s| {
                    Box::new(TlsConnection::new(s, self.conn.clone())) as BoxedStream
                })
        }
    }
    /// A trait providing necessary functions to handle a connection
    pub trait Connection: Sized + Send {
        type Body: 'static + GetAsSlice + Send;
        type Responder: 'static + Responder;
        fn new(conn: BoxedStream) -> Result<Self, std::io::Error>;
        fn set_timeout(
            &mut self,
            timeout: Option<Duration>,
        ) -> Result<(), std::io::Error>;
        /// Reading of the body of the previous frame may occur on subsequent calls depending on
        /// implementation
        fn next_frame(
            &mut self,
        ) -> Result<(Request<Self::Body>, Self::Responder), std::io::Error>;
        fn upgrade(self) -> BoxedStream;
    }
    /// A trait providing necessary functionality to respond to a connection
    pub trait Responder: Sized + Send {
        /// Write bytes of response to the underlying writer. This can be expected to be the full
        /// response
        fn write_bytes(self, bytes: Vec<u8>) -> Result<(), std::io::Error>;
        fn respond(
            self,
            response: impl Into<RawResponse>,
        ) -> Result<(), std::io::Error> {
            let response: RawResponse = response.into();
            let bytes = response.into_raw_bytes();
            self.write_bytes(bytes)
        }
    }
    /// HTTP 1.1
    pub struct Http1 {
        conn: BoxedStream,
        next_writer: Option<Receiver<BoxedStream>>,
        unfinished: Option<(usize, Sender<Vec<u8>>)>,
    }
    impl Http1 {
        fn next_writer(
            &mut self,
        ) -> Result<SequentialWriter<BoxedStream>, std::io::Error> {
            Ok(
                match self.next_writer.take() {
                    Some(writer) => {
                        let (writer, receiver) = SequentialWriter::new(
                            sequential_writer::State::Waiting(writer),
                        );
                        self.next_writer = Some(receiver);
                        writer
                    }
                    None => {
                        let (writer, receiver) = SequentialWriter::new(
                            sequential_writer::State::Writer(self.conn.try_clone()?),
                        );
                        self.next_writer = Some(receiver);
                        writer
                    }
                },
            )
        }
    }
    impl Connection for Http1 {
        type Body = Lazy<Vec<u8>>;
        type Responder = SequentialWriter<BoxedStream>;
        fn new(conn: BoxedStream) -> Result<Self, std::io::Error> {
            Ok(Self {
                conn,
                next_writer: None,
                unfinished: None,
            })
        }
        fn set_timeout(
            &mut self,
            timeout: Option<Duration>,
        ) -> Result<(), std::io::Error> {
            self.conn.set_timeout(timeout)
        }
        fn next_frame(
            &mut self,
        ) -> Result<(Request<Self::Body>, Self::Responder), std::io::Error> {
            if let Some((len, sender)) = self.unfinished.take() {
                let mut buf = ::alloc::vec::from_elem(0, len);
                if self.conn.read_exact(&mut buf).is_err() {
                    let _ = sender.send(::alloc::vec::Vec::new());
                }
                let _ = sender.send(buf);
            }
            let req = take_request(&mut BufReader::new(&mut self.conn))
                .map_err(|_| {
                    std::io::Error::new(
                        ErrorKind::Other,
                        "Failed to parse request from stream",
                    )
                })?;
            let body_len = req
                .headers()
                .get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            let (lazy, sender) = Lazy::new();
            self.unfinished = Some((body_len, sender));
            Ok((req.map(|_| lazy), self.next_writer()?))
        }
        fn upgrade(self) -> BoxedStream {
            self.conn
        }
    }
    impl<W> Responder for SequentialWriter<W>
    where
        W: Write + Send + Sync,
    {
        fn write_bytes(self, bytes: Vec<u8>) -> Result<(), std::io::Error> {
            self.send(&bytes)
        }
    }
}
pub mod framework {
    //! This module provides the application entry point.
    //!
    use std::{
        marker::PhantomData, net::{TcpListener, ToSocketAddrs},
        sync::Arc,
    };
    use crate::{
        connection::Connection, layers::{BoxLayer, DefaultResponseGroup, Layer},
        routing::Router, tasks::TaskPool, type_cache::TypeCache, Request, Response,
    };
    #[cfg(not(feature = "tls"))]
    use crate::tasks::ConnectionTask;
    /// Main application entry point. Construct this type to run your application.
    pub struct App {
        router: Router,
        request_layer: BoxLayer<Request>,
        response_layer: BoxLayer<Response>,
        type_cache: TypeCache,
    }
    impl App {
        /// Constructs a new application
        pub fn builder(scope: Router) -> Self {
            Self {
                router: scope.into(),
                request_layer: Box::new(()),
                response_layer: Box::new(DefaultResponseGroup::new()),
                type_cache: TypeCache::new(),
            }
        }
        /// Overrides the default request `Layer` if one is set
        pub fn request_layer(
            mut self,
            layer: impl 'static + Layer<Request> + Send + Sync,
        ) -> Self {
            self.request_layer = Box::new(layer);
            self
        }
        /// Overrides the default response `Layer` if one is set
        pub fn response_layer(
            mut self,
            layer: impl 'static + Layer<Response> + Send + Sync,
        ) -> Self {
            self.response_layer = Box::new(layer);
            self
        }
        /// Sets the cache to be used by the application
        pub fn cache(mut self, cache: TypeCache) -> Self {
            self.type_cache = cache;
            self
        }
        /// Executes the application. This will currently never return.
        pub fn run<C>(self, address: impl ToSocketAddrs)
        where
            C: 'static + Connection,
        {
            let incoming = TcpListener::bind(address)
                .expect("Could not bind to local address");
            let type_cache = Arc::new(self.type_cache);
            let router = Arc::new(self.router);
            let request_layer = Arc::new(self.request_layer);
            let response_layer = Arc::new(self.response_layer);
            let task_pool = TaskPool::new();
            loop {
                let Ok((stream, _addr)) = incoming.accept() else {
                    continue;
                };
                #[cfg(not(feature = "tls"))]
                let task = ConnectionTask::<C> {
                    task_pool: task_pool.clone(),
                    cache: type_cache.clone(),
                    stream,
                    router: router.clone(),
                    response_layer: response_layer.clone(),
                    request_layer: request_layer.clone(),
                    phantom_data: PhantomData,
                };
                task_pool.send_task(task);
            }
        }
    }
}
pub mod get_as_slice {
    pub trait GetAsSlice {
        fn get_as_slice(&self) -> &[u8];
    }
}
pub mod http_utils {
    //! This module provides http utility traits and functions for parsing and handling Requests and
    //! Responses
    use http::{Request, Response, Version};
    use std::io::{BufRead, Write};
    use crate::action::RawResponse;
    /// Errors while parsing requests.
    pub enum ParseError {
        MalformedRequest,
        ReadError,
        InvalidProtocolVer,
        InvalidRequestParts,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for ParseError {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    ParseError::MalformedRequest => "MalformedRequest",
                    ParseError::ReadError => "ReadError",
                    ParseError::InvalidProtocolVer => "InvalidProtocolVer",
                    ParseError::InvalidRequestParts => "InvalidRequestParts",
                },
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for ParseError {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for ParseError {
        #[inline]
        fn eq(&self, other: &ParseError) -> bool {
            let __self_tag = ::core::intrinsics::discriminant_value(self);
            let __arg1_tag = ::core::intrinsics::discriminant_value(other);
            __self_tag == __arg1_tag
        }
    }
    impl std::fmt::Display for ParseError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ParseError::MalformedRequest => {
                    f.write_fmt(format_args!("Malformed Request"))
                }
                ParseError::ReadError => f.write_fmt(format_args!("Read Error")),
                ParseError::InvalidProtocolVer => {
                    f.write_fmt(format_args!("Invalid Protocol"))
                }
                ParseError::InvalidRequestParts => {
                    f.write_fmt(format_args!("Invalid Request Parts"))
                }
            }
        }
    }
    impl std::error::Error for ParseError {}
    /// `Version` Extension trait
    pub trait VersionExt: Sized {
        /// Parse `Version` from a `&str`. Returns `Err` if the `&str` isn't a valid version of the HTTP protocol
        fn parse_version(s: &str) -> Result<Self, ParseError>;
        /// Convert a `Version` to a `String`
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
    /// Read a request from a source
    pub fn take_request<R>(reader: &mut R) -> Result<Request<()>, ParseError>
    where
        R: BufRead,
    {
        let mut lines = reader.lines();
        let line = lines
            .next()
            .ok_or(ParseError::MalformedRequest)?
            .map_err(|_| ParseError::ReadError)?;
        let mut parts = line.split(' ');
        let method = parts.next().ok_or(ParseError::MalformedRequest)?;
        let uri = parts.next().ok_or(ParseError::MalformedRequest)?;
        let version = parts.next().ok_or(ParseError::MalformedRequest)?;
        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .version(Version::parse_version(version)?);
        while let Some(line) = lines
            .next()
            .transpose()
            .map_err(|_| ParseError::ReadError)?
        {
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
    fn parse_response_line_into_buf<T>(
        buf: &mut Vec<u8>,
        request: &Response<T>,
    ) -> Result<(), std::io::Error> {
        buf.write_fmt(
            format_args!("{0} {1}\r\n", request.version().to_string(), request.status()),
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
        fn into_raw_response(self) -> RawResponse;
    }
    impl<T> ResponseExt for Response<T>
    where
        T: IntoRawBytes,
    {
        fn into_raw_response(self) -> RawResponse {
            self.map(IntoRawBytes::into_raw_bytes)
        }
    }
}
pub mod layers {
    use http::HeaderValue;
    use crate::{Request, Response};
    pub type BoxLayer<T> = Box<dyn 'static + Layer<T> + Send + Sync>;
    /// A collection of `Layers` to be executed in sequence.
    pub struct LayerGroup<I> {
        layers: Vec<BoxLayer<I>>,
    }
    impl<I> LayerGroup<I> {
        /// Constructs a new `LayerGroup`
        pub fn new() -> Self {
            LayerGroup { layers: Vec::new() }
        }
        /// pushes a new `Layer` to the stack
        pub fn add_layer(
            mut self,
            layer: impl 'static + Layer<I> + Send + Sync,
        ) -> Self {
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
        pub fn new() -> LayerGroup<Response> {
            let group = LayerGroup::new().add_layer(SetContentLength);
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
            let value = HeaderValue::from_str(
                    &{
                        let res = ::alloc::fmt::format(format_args!("{0}", bytes));
                        res
                    },
                )
                .expect("Failed to parse length as HeaderValue");
            data.headers_mut().insert("content-length", value);
        }
    }
    /// Sets the date header of all outgoing requests
    #[cfg(feature = "date")]
    pub struct SetDate;
    #[cfg(feature = "date")]
    impl Layer<Response> for SetDate {
        fn execute(&self, data: &mut Response) {
            let date = chrono::Utc::now().to_rfc2822();
            let value = HeaderValue::from_str(&date)
                .expect("Failed to convert date to header value");
            data.headers_mut().insert("date", value);
        }
    }
}
pub mod resolve {
    use crate::{
        action::RawResponse, routing::Captures, type_cache::TypeCacheKey, RequestState,
    };
    /// `Resolve` is a trait used to construct values needed to call a given `System`. All parameters
    /// of a `System` must implement `Resolve` to be valid.
    pub trait Resolve<'a>: Sized {
        type Output: 'a;
        fn resolve(
            ctx: &'a RequestState,
            captures: &mut Captures,
        ) -> ResolveGuard<Self::Output>;
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
    impl<T> From<Option<T>> for ResolveGuard<T> {
        fn from(value: Option<T>) -> Self {
            match value {
                Some(v) => ResolveGuard::Value(v),
                None => ResolveGuard::None,
            }
        }
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
    /// "Query" a value from the global_cache of the `RequestState` and clone it.
    pub struct Query<K>(
        pub K::Value,
    )
    where
        K: TypeCacheKey;
    impl<'a, K> Resolve<'a> for Query<K>
    where
        K: TypeCacheKey,
        K::Value: Clone,
    {
        type Output = Self;
        fn resolve(
            ctx: &'a RequestState,
            captures: &mut Captures,
        ) -> ResolveGuard<Self> {
            ctx.global_cache.get::<K>().map(|v| Query(v.clone())).into()
        }
    }
    /// Consumes the next part of the url `path_iter`. Note that this will happen on call to its
    /// `resolve` method so ordering of parameters matter. Place any necessary guards before this
    /// method.
    pub struct UrlPart(pub String);
    impl<'a> Resolve<'a> for UrlPart {
        type Output = Self;
        fn resolve(
            ctx: &'a RequestState,
            captures: &mut Captures,
        ) -> ResolveGuard<Self> {
            let Some(part) = captures.pop_front() else {
                return ResolveGuard::None;
            };
            ResolveGuard::Value(UrlPart(part))
        }
    }
    /// Collect the entire remaining url into a `Vec` Note that this will happen on call to its
    /// `resolve` method so ordering of parameters matter. Place any necessary guards before this
    /// method.
    pub struct UrlCollect(pub Vec<String>);
    impl<'a> Resolve<'a> for UrlCollect {
        type Output = Self;
        fn resolve(
            ctx: &'a RequestState,
            captures: &mut Captures,
        ) -> ResolveGuard<Self> {
            let mut new = Vec::new();
            while let Some(part) = captures.pop_front() {
                new.push(part)
            }
            ResolveGuard::Value(UrlCollect(new))
        }
    }
    impl<'a, 'b> Resolve<'a> for &'b [u8] {
        type Output = &'a [u8];
        fn resolve(
            ctx: &'a RequestState,
            captures: &mut Captures,
        ) -> ResolveGuard<Self::Output> {
            ResolveGuard::Value(ctx.request.body().get_as_slice())
        }
    }
    impl<'a, 'b> Resolve<'a> for &'b str {
        type Output = &'a str;
        fn resolve(
            ctx: &'a RequestState,
            captures: &mut Captures,
        ) -> ResolveGuard<Self::Output> {
            std::str::from_utf8(ctx.request.body().get_as_slice()).ok().into()
        }
    }
}
pub mod routing {
    use std::{borrow::BorrowMut, collections::VecDeque};
    use crate::handler::{Handler, InsertHandler};
    pub type Captures = VecDeque<String>;
    struct Node {
        handler: Option<Handler>,
        children: Vec<(Pattern, Node)>,
    }
    impl Node {
        fn new() -> Self {
            Self {
                handler: None,
                children: Vec::new(),
            }
        }
    }
    enum Pattern {
        Exact(String),
        Capture,
        Collect,
    }
    impl Pattern {
        fn new(s: &str) -> Self {
            if s.starts_with(":") {
                Pattern::Capture
            } else if s == "*" {
                Pattern::Collect
            } else {
                Pattern::Exact(s.to_string())
            }
        }
        fn exact(&self, rhs: &str) -> bool {
            match self {
                Pattern::Exact(s) => s == rhs,
                Pattern::Capture if rhs.starts_with(":") => true,
                Pattern::Collect if rhs == "*" => true,
                _ => false,
            }
        }
    }
    pub struct Router {
        root: Node,
    }
    impl Router {
        pub fn new() -> Self {
            Self { root: Node::new() }
        }
        pub fn add_route<T>(
            mut self,
            path: &str,
            handler: impl InsertHandler<T>,
        ) -> Self {
            let mut cursor = &mut self.root;
            for segment in path.split("/") {
                if segment.is_empty() {
                    continue;
                }
                if let Some(idx) = cursor
                    .children
                    .iter()
                    .position(|i| i.0.exact(segment))
                {
                    cursor = &mut cursor.children[idx].1;
                } else {
                    cursor.children.push((Pattern::new(segment), Node::new()));
                    {
                        ::std::io::_print(format_args!("{0}\n", cursor.children.len()));
                    };
                    cursor = cursor.children.last_mut().unwrap().1.borrow_mut();
                }
            }
            let mut new = Handler::new();
            handler.insert_to_handler(&mut new);
            cursor.handler = Some(new);
            self
        }
        pub fn route(&self, path: &str) -> Option<(&Handler, Captures)> {
            let mut captured = ::alloc::vec::Vec::new();
            let mut cursor = &self.root;
            let mut iter = path.split("/");
            'outer: while let Some(segment) = iter.next() {
                if segment.is_empty() {
                    continue;
                }
                let mut matched = false;
                for (pattern, node) in cursor.children.iter() {
                    match pattern {
                        Pattern::Exact(s) if s == segment => {
                            cursor = &node;
                            matched = true;
                            break;
                        }
                        Pattern::Capture => {
                            captured.push(segment.to_string());
                            cursor = node;
                            matched = true;
                            break;
                        }
                        Pattern::Collect => {
                            captured.push(segment.to_string());
                            captured.extend(iter.map(String::from));
                            cursor = node;
                            break 'outer;
                        }
                        _ => {}
                    }
                }
                if !matched {
                    return None;
                }
            }
            cursor.handler.as_ref().map(|i| (i, VecDeque::from(captured)))
        }
    }
}
pub mod systems {
    use crate::{
        action::{Action, IntoAction},
        resolve::{Resolve, ResolveGuard},
        tasks::RequestState,
    };
    use std::collections::VecDeque;
    #[doc(hidden)]
    pub trait System<'a, T> {
        fn run(self, ctx: &'a RequestState, captures: VecDeque<String>) -> Action;
    }
    #[doc(hidden)]
    pub struct DynSystem {
        inner: Box<
            dyn Fn(&RequestState, VecDeque<String>) -> Action + 'static + Send + Sync,
        >,
    }
    impl DynSystem {
        pub fn new<A>(
            system: impl for<'a> System<'a, A> + 'static + Send + Sync + Copy,
        ) -> Self {
            DynSystem {
                inner: Box::new(move |ctx, captures| system.run(ctx, captures)),
            }
        }
        pub fn call(&self, ctx: &RequestState, captures: VecDeque<String>) -> Action {
            (self.inner)(ctx, captures)
        }
    }
    #[doc(hidden)]
    pub trait IntoDynSystem<T> {
        fn into_dyn_system(self) -> DynSystem;
    }
    impl<T, A> IntoDynSystem<A> for T
    where
        T: for<'a> System<'a, A> + 'static + Send + Sync + Copy,
    {
        fn into_dyn_system(self) -> DynSystem {
            DynSystem::new(self)
        }
    }
    impl<
        'a,
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
        'a,
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
            ) -> RESPONSE
            + Fn(
                A::Output,
                B::Output,
                C::Output,
                D::Output,
                E::Output,
                F::Output,
                G::Output,
                H::Output,
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        A: Resolve<'a>,
        B: Resolve<'a>,
        C: Resolve<'a>,
        D: Resolve<'a>,
        E: Resolve<'a>,
        F: Resolve<'a>,
        G: Resolve<'a>,
        H: Resolve<'a>,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let A = match A::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let B = match B::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let C = match C::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let D = match D::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let E = match E::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let F = match F::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
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
            r.action()
        }
    }
    impl<
        'a,
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
        'a,
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
            ) -> RESPONSE
            + Fn(
                B::Output,
                C::Output,
                D::Output,
                E::Output,
                F::Output,
                G::Output,
                H::Output,
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        B: Resolve<'a>,
        C: Resolve<'a>,
        D: Resolve<'a>,
        E: Resolve<'a>,
        F: Resolve<'a>,
        G: Resolve<'a>,
        H: Resolve<'a>,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let B = match B::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let C = match C::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let D = match D::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let E = match E::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let F = match F::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
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
            r.action()
        }
    }
    impl<
        'a,
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
        'a,
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
            ) -> RESPONSE
            + Fn(
                C::Output,
                D::Output,
                E::Output,
                F::Output,
                G::Output,
                H::Output,
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        C: Resolve<'a>,
        D: Resolve<'a>,
        E: Resolve<'a>,
        F: Resolve<'a>,
        G: Resolve<'a>,
        H: Resolve<'a>,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let C = match C::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let D = match D::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let E = match E::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let F = match F::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
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
            r.action()
        }
    }
    impl<
        'a,
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
        'a,
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
            ) -> RESPONSE
            + Fn(
                D::Output,
                E::Output,
                F::Output,
                G::Output,
                H::Output,
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        D: Resolve<'a>,
        E: Resolve<'a>,
        F: Resolve<'a>,
        G: Resolve<'a>,
        H: Resolve<'a>,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let D = match D::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let E = match E::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let F = match F::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
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
            r.action()
        }
    }
    impl<
        'a,
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
        'a,
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
            ) -> RESPONSE
            + Fn(
                E::Output,
                F::Output,
                G::Output,
                H::Output,
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        E: Resolve<'a>,
        F: Resolve<'a>,
        G: Resolve<'a>,
        H: Resolve<'a>,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let E = match E::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let F = match F::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
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
            r.action()
        }
    }
    impl<
        'a,
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
    > System<
        'a,
        (RESPONSE, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z),
    > for BASE
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
            ) -> RESPONSE
            + Fn(
                F::Output,
                G::Output,
                H::Output,
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        F: Resolve<'a>,
        G: Resolve<'a>,
        H: Resolve<'a>,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let F = match F::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let G = match G::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)>
    for BASE
    where
        BASE: Fn(G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                G::Output,
                H::Output,
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        G: Resolve<'a>,
        H: Resolve<'a>,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let G = match G::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let H = match H::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)>
    for BASE
    where
        BASE: Fn(H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                H::Output,
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        H: Resolve<'a>,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let H = match H::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let I = match I::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)>
    for BASE
    where
        BASE: Fn(I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                I::Output,
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        I: Resolve<'a>,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let I = match I::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let J = match J::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                J::Output,
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        J: Resolve<'a>,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let J = match J::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let K = match K::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                K::Output,
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        K: Resolve<'a>,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let K = match K::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let L = match L::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                L::Output,
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        L: Resolve<'a>,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let L = match L::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let M = match M::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                M::Output,
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        M: Resolve<'a>,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let M = match M::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let N = match N::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(N, O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                N::Output,
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        N: Resolve<'a>,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let N = match N::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let O = match O::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, O, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(O, P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                O::Output,
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        O: Resolve<'a>,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let O = match O::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let P = match P::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(O, P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, P, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(P, Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                P::Output,
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        P: Resolve<'a>,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let P = match P::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Q = match Q::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(P, Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, Q, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(Q, R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                Q::Output,
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        Q: Resolve<'a>,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let Q = match Q::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(Q, R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, R, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(R, S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                R::Output,
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        R: Resolve<'a>,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let R = match R::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(R, S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
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
    > System<'a, (RESPONSE, S, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(S, T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                S::Output,
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        S: Resolve<'a>,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let S = match S::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(S, T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<
        'a,
        RESPONSE,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        BASE,
    > System<'a, (RESPONSE, T, U, V, W, X, Y, Z)> for BASE
    where
        BASE: Fn(T, U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                T::Output,
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        T: Resolve<'a>,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let T = match T::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(T, U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<'a, RESPONSE, U, V, W, X, Y, Z, BASE> System<'a, (RESPONSE, U, V, W, X, Y, Z)>
    for BASE
    where
        BASE: Fn(U, V, W, X, Y, Z) -> RESPONSE
            + Fn(
                U::Output,
                V::Output,
                W::Output,
                X::Output,
                Y::Output,
                Z::Output,
            ) -> RESPONSE,
        U: Resolve<'a>,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let U = match U::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(U, V, W, X, Y, Z);
            r.action()
        }
    }
    impl<'a, RESPONSE, V, W, X, Y, Z, BASE> System<'a, (RESPONSE, V, W, X, Y, Z)>
    for BASE
    where
        BASE: Fn(V, W, X, Y, Z) -> RESPONSE
            + Fn(V::Output, W::Output, X::Output, Y::Output, Z::Output) -> RESPONSE,
        V: Resolve<'a>,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let V = match V::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(V, W, X, Y, Z);
            r.action()
        }
    }
    impl<'a, RESPONSE, W, X, Y, Z, BASE> System<'a, (RESPONSE, W, X, Y, Z)> for BASE
    where
        BASE: Fn(W, X, Y, Z) -> RESPONSE
            + Fn(W::Output, X::Output, Y::Output, Z::Output) -> RESPONSE,
        W: Resolve<'a>,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let W = match W::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(W, X, Y, Z);
            r.action()
        }
    }
    impl<'a, RESPONSE, X, Y, Z, BASE> System<'a, (RESPONSE, X, Y, Z)> for BASE
    where
        BASE: Fn(X, Y, Z) -> RESPONSE + Fn(X::Output, Y::Output, Z::Output) -> RESPONSE,
        X: Resolve<'a>,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let X = match X::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(X, Y, Z);
            r.action()
        }
    }
    impl<'a, RESPONSE, Y, Z, BASE> System<'a, (RESPONSE, Y, Z)> for BASE
    where
        BASE: Fn(Y, Z) -> RESPONSE + Fn(Y::Output, Z::Output) -> RESPONSE,
        Y: Resolve<'a>,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let Y = match Y::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(Y, Z);
            r.action()
        }
    }
    impl<'a, RESPONSE, Z, BASE> System<'a, (RESPONSE, Z)> for BASE
    where
        BASE: Fn(Z) -> RESPONSE + Fn(Z::Output) -> RESPONSE,
        Z: Resolve<'a>,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            #[allow(non_snake_case)]
            let Z = match Z::resolve(&ctx, &mut captures) {
                ResolveGuard::Value(v) => v,
                ResolveGuard::None => return Action::None,
                ResolveGuard::Respond(r) => return Action::Respond(r),
            };
            let r = self(Z);
            r.action()
        }
    }
    impl<'a, RESPONSE, BASE> System<'a, (RESPONSE,)> for BASE
    where
        BASE: Fn() -> RESPONSE + Fn() -> RESPONSE,
        RESPONSE: IntoAction,
    {
        #[allow(unused)]
        fn run(
            self,
            mut ctx: &'a RequestState,
            mut captures: VecDeque<String>,
        ) -> Action {
            let r = self();
            r.action()
        }
    }
}
pub mod type_cache {
    use std::{
        any::{Any, TypeId},
        collections::HashMap,
    };
    type Value = Box<dyn Any + Sync + Send>;
    /// This trait allows a type to be used as a key in a `TypeCache`
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
pub use action::{Action, IntoResponse};
pub use connection::Http1;
pub use framework::App;
pub use http_utils::IntoRawBytes;
pub use layers::{DefaultResponseGroup, Layer};
pub use resolve::{Resolve, ResolveGuard};
pub use routing::Router;
pub use tasks::{PathIter, RequestState};
pub use type_cache::{TypeCache, TypeCacheKey};
pub use routing::Captures;
pub use http;
/// Request type used by most of `foxhole`
pub type Request = tasks::BoxedBodyRequest;
/// Response type used by most of `foxhole`
pub type Response = action::RawResponse;
