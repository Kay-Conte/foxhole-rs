use std::{
    collections::VecDeque,
    marker::PhantomData,
    net::TcpStream,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Condvar, Mutex,
    },
    time::Duration,
};

use http::Request;

use crate::{
    connection::{Connection, Responder},
    get_as_slice::GetAsSlice,
    layers::BoxLayer,
    type_cache::TypeCache,
    Action, Response, Router,
};

#[cfg(feature = "tls")]
use std::sync::RwLock;

#[cfg(feature = "tls")]
use rustls::{ServerConfig, ServerConnection};

#[cfg(feature = "tls")]
use crate::tls_connection::TlsConnection;

const MIN_THREADS: usize = 4;
const TIMEOUT: u64 = 5;

/// Request with a boxed body implementing `GetAsSlice` This is the standard request type
/// throughout the library
pub type BoxedBodyRequest = Request<Box<dyn 'static + GetAsSlice + Send>>;

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
            let r = request.map(|i| {
                let b: Box<dyn 'static + GetAsSlice + Send> = Box::new(i);

                b
            });

            let mut should_close: bool = true;

            if let Some(header) = r.headers().get("connection").and_then(|i| i.to_str().ok()) {
                #[cfg(feature = "websocket")]
                if header.to_lowercase() == "upgrade" {
                    self.task_pool.send_task(RequestTask {
                        cache: self.cache.clone(),
                        upgrade: Some(connection),
                        request: r,
                        responder,
                        router: self.router.clone(),
                        request_layer: self.request_layer.clone(),
                        response_layer: self.response_layer.clone(),
                    });

                    break;
                }

                should_close = header != "keep-alive";
            }

            self.task_pool.send_task(RequestTask::<_, C> {
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

#[cfg(feature = "tls")]
pub(crate) struct SecuredConnectionTask<C> {
    pub task_pool: TaskPool,

    pub cache: Arc<TypeCache>,

    pub stream: TcpStream,

    pub router: Arc<Router>,

    pub tls_config: Arc<ServerConfig>,

    pub request_layer: Arc<BoxLayer<crate::Request>>,
    pub response_layer: Arc<BoxLayer<Response>>,

    pub phantom_data: PhantomData<C>,
}

#[cfg(feature = "tls")]
impl<C> Task for SecuredConnectionTask<C>
where
    C: 'static + Connection,
{
    fn run(mut self: Box<Self>) {
        let Ok(mut connection) = ServerConnection::new(self.tls_config.clone()) else {
            return;
        };

        match connection.complete_io(&mut self.stream) {
            Ok(_) => {}
            Err(_) => {
                return;
            }
        };

        let Ok(mut connection) = C::new(Box::new(TlsConnection::new(
            self.stream,
            Arc::new(RwLock::new(connection)),
        ))) else {
            return;
        };

        connection
            .set_timeout(Some(Duration::from_secs(TIMEOUT)))
            .expect("Shouldn't fail unless duration is 0");

        while let Ok((request, responder)) = connection.next_frame() {
            let r = request.map(|i| {
                let b: Box<dyn 'static + GetAsSlice + Send> = Box::new(i);

                b
            });

            let mut should_close: bool = true;

            if let Some(header) = r.headers().get("connection").and_then(|i| i.to_str().ok()) {
                if header.to_lowercase() == "upgrade" {
                    self.task_pool.send_task(RequestTask {
                        cache: self.cache.clone(),
                        upgrade: Some(connection),
                        request: r,
                        responder,
                        router: self.router.clone(),
                        request_layer: self.request_layer.clone(),
                        response_layer: self.response_layer.clone(),
                    });

                    break;
                }

                should_close = header != "keep-alive";
            }

            self.task_pool.send_task(RequestTask::<_, C> {
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
            // TODO handle fallback
            return;
        };

        let Some(system) = handler.get(self.request.method()) else {
            // TODO handle fallback
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
            #[cfg(feature = "websocket")]
            Action::Upgrade(r, f) => {
                // Todo move this handling to the constructor of `Action::Upgrade`
                let Some(connection) = self.upgrade else {
                    return;
                };

                let response = r(&ctx.request);
                let _ = self.responder.respond(response);

                f(connection.upgrade());

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

#[derive(Clone)]
pub(crate) struct TaskPool {
    shared: Arc<Shared>,
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
    fn spawn_thread(&self, should_cull: bool, initial_task: Option<Box<dyn Task + Send>>) {
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
                        // Make sure not to bloat waiting count
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
