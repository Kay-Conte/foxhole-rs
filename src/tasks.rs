use std::{
    collections::VecDeque,
    iter::Peekable,
    marker::PhantomData,
    net::TcpStream,
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
    get_as_slice::GetAsSlice,
    routing::Router,
    type_cache::TypeCacheShared,
    IntoResponse,
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

/// The url iterator type used by the library. This can be found in `RequestState` accessed via the
/// `Resolve` trait
pub type PathIter<'a> = Peekable<Split<'a, &'static str>>;

pub(crate) trait Task {
    fn run(self: Box<Self>);
}

pub(crate) struct ConnectionTask<C> {
    pub task_pool: TaskPool,

    /// An application global type cache
    pub cache: TypeCacheShared,

    pub stream: TcpStream,

    /// A handle to the applications router tree
    pub router: Arc<Router>,

    pub phantom_data: PhantomData<C>,
}

impl<C> Task for ConnectionTask<C>
where
    C: Connection,
{
    fn run(self: Box<Self>) {
        let Ok(mut connection) = C::new(Box::new(self.stream)) else {
            return;
        };

        connection
            .set_read_timeout(Some(Duration::from_secs(TIMEOUT)))
            .expect("Shouldn't fail unless duration is 0");

        while let Ok((request, responder)) = connection.next_frame() {
            let r = request.map(|i| {
                let b: Box<dyn 'static + GetAsSlice + Send> = Box::new(i);

                b
            });

            self.task_pool.send_task(RequestTask {
                cache: self.cache.clone(),
                request: r,
                responder,
                router: self.router.clone(),
            });
        }
    }
}

#[cfg(feature = "tls")]
pub(crate) struct SecuredConnectionTask<C> {
    pub task_pool: TaskPool,

    pub cache: TypeCacheShared,

    pub stream: TcpStream,

    pub router: Arc<Router>,

    pub tls_config: Arc<ServerConfig>,

    pub phantom_data: PhantomData<C>,
}

#[cfg(feature = "tls")]
impl<C> Task for SecuredConnectionTask<C>
where
    C: Connection,
{
    fn run(mut self: Box<Self>) {
        let Ok(mut connection) = ServerConnection::new(self.tls_config.clone()) else {
            println!("Err");
            return;
        };

        match connection.complete_io(&mut self.stream) {
            Ok(_) => {}
            Err(e) => {
                println!("{}", e);
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
            .set_read_timeout(Some(Duration::from_secs(TIMEOUT)))
            .expect("Shouldn't fail unless duration is 0");

        while let Ok((request, responder)) = connection.next_frame() {
            let r = request.map(|i| {
                let b: Box<dyn 'static + GetAsSlice + Send> = Box::new(i);

                b
            });

            self.task_pool.send_task(RequestTask {
                cache: self.cache.clone(),
                request: r,
                responder,
                router: self.router.clone(),
            });
        }
    }
}

pub(crate) struct RequestTask<R> {
    pub cache: TypeCacheShared,

    pub request: BoxedBodyRequest,

    pub responder: R,

    /// A handle to the applications router tree
    pub router: Arc<Router>,
}

impl<R> Task for RequestTask<R>
where
    R: Responder,
{
    fn run(self: Box<Self>) {
        let path = self.request.uri().path().to_owned();

        let mut path_iter = path.split("/").peekable();

        path_iter.next();

        let mut ctx = RequestState {
            global_cache: self.cache.clone(),
            request: self.request,
        };

        self.router.get_request_layer().execute(&mut ctx.request);

        let mut cursor = self.router.scope();

        loop {
            for system in cursor.systems() {
                if let Some(mut r) = system.call(&ctx, &mut path_iter) {
                    self.router.get_response_layer().execute(&mut r);

                    self.responder.respond(r).unwrap();

                    return;
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
    pub global_cache: TypeCacheShared,
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
