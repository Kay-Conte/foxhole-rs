use std::{
    collections::VecDeque,
    io::{BufReader, Read},
    iter::Peekable,
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
    http_utils::{take_request, IntoRawBytes},
    lazy::Lazy,
    routing::Route,
    sequential_writer::{self, SequentialWriter},
    type_cache::TypeCacheShared,
    IntoResponse, systems::Action,
};

const TIMEOUT: u64 = 5;

pub type PathIter<'a> = Peekable<Split<'a, &'static str>>;

type RawData = Vec<u8>;

pub trait Task {
    fn run(self: Box<Self>);
}

pub struct ConnectionTask {
    pub task_pool: TaskPool,

    /// An application global type cache
    pub cache: TypeCacheShared,

    pub stream: TcpStream,

    /// A handle to the applications router tree
    pub router: Arc<Route>,
}

impl Task for ConnectionTask {
    fn run(self: Box<Self>) {
        self.stream
            .set_read_timeout(Some(Duration::from_secs(TIMEOUT)))
            .expect("Shouldn't fail unless duration is 0");

        let Ok(writer) = self.stream.try_clone() else {
            // Stream closed early
            return;
        };

        let mut writer = SequentialWriter::new(sequential_writer::State::Writer(writer));

        let mut reader = BufReader::new(self.stream);

        while let Ok(req) = take_request(&mut reader) {
            let (lazy, sender) = Lazy::new();

            let body_len = req
                .headers()
                .get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);

            self.task_pool.send_task(RequestTask {
                cache: self.cache.clone(),
                request: req.map(|_| lazy),
                writer: writer.0,
                router: self.router.clone(),
            });

            let mut buf = vec![0; body_len];

            if reader.read_exact(&mut buf).is_err() {
                // Avoid blocking request tasks awaiting a body that doesn't exist.
                let _ = sender.send(vec![]);

                return;
            };

            // This will only error if the `Request` was already responded to before we finish
            // reading the body.
            let _ = sender.send(buf);

            writer = SequentialWriter::new(sequential_writer::State::Waiting(writer.1));
        }
    }
}

pub struct RequestTask {
    pub cache: TypeCacheShared,

    pub request: Request<Lazy<RawData>>,

    pub writer: SequentialWriter<TcpStream>,

    /// A handle to the applications router tree
    pub router: Arc<Route>,
}

impl Task for RequestTask {
    fn run(self: Box<Self>) {
        let path = self.request.uri().path().to_owned();

        let mut path_iter = path.split("/").peekable();

        path_iter.next();

        let ctx = RequestState {
            global_cache: self.cache.clone(),
            request: self.request,
        };

        let mut cursor = self.router.as_ref();

        loop {
            for system in cursor.systems() {

                match system.call(&ctx, &mut path_iter) {
                    Action::Response(r) => {
                        self.writer.send(&r.into_raw_bytes()).unwrap();

                        return;
                    }
                    Action::Upgrade(f) => f(),
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

        let _ = self
            .writer
            .send(&404u16.response().into_raw_bytes());
    }
}

pub struct RequestState {
    pub global_cache: TypeCacheShared,
    pub request: Request<Lazy<RawData>>,
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
pub struct TaskPool {
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
    fn spawn_thread(&self) {
        let shared = self.shared.clone();

        std::thread::spawn(move || {
            loop {
                let mut pool = shared.pool.lock().unwrap();

                shared.waiting();

                let (new, timeout) = shared
                    .condvar
                    .wait_timeout(pool, Duration::from_secs(5))
                    .unwrap();

                if timeout.timed_out() {
                    // Make sure not to bloat waiting count
                    break;
                }

                pool = new;

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
        self.shared.pool.lock().unwrap().push_back(Box::new(task));

        if self.shared.waiting_tasks.load(Ordering::Acquire) == 0 {
            self.spawn_thread();
        }

        self.shared.condvar.notify_one();
    }
}
