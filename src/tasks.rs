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
    type_cache::{TypeCache, TypeCacheShared},
    MaybeIntoResponse,
};

const MIN_THREADS: usize = 4;
const TIMEOUT: u64 = 5;

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

        // FIXME panics if stream closed early
        let mut writer = SequentialWriter::new(sequential_writer::State::Writer(
            self.stream.try_clone().unwrap(),
        ));

        let mut reader = BufReader::new(self.stream);

        while let Ok(req) = take_request(&mut reader) {
            println!("Request");

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
                // Avoid blocking request tasks awaiting a body
                sender.send(vec![]).unwrap();

                return;
            };

            sender.send(buf).unwrap();

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

        let mut ctx = RequestState {
            global_cache: self.cache.clone(),
            local_cache: TypeCache::new(),
            request: self.request,
            path_iter,
        };

        let mut cursor = self.router.as_ref();

        loop {
            for system in cursor.systems() {
                if let Some(r) = system.call(&mut ctx) {
                    self.writer.send(&r.into_raw_bytes()).unwrap();

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

        let _ = self
            .writer
            .send(&404u16.maybe_response().unwrap().into_raw_bytes());
    }
}

pub struct RequestState<'a> {
    pub global_cache: TypeCacheShared,
    pub local_cache: TypeCache,
    pub request: Request<Lazy<RawData>>,
    pub path_iter: Peekable<Split<'a, &'static str>>,
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
        self.shared.pool.lock().unwrap().push_back(Box::new(task));

        if self.shared.waiting_tasks.load(Ordering::Acquire) < MIN_THREADS {
            self.spawn_thread(true);
        }

        self.shared.condvar.notify_one();
    }
}
