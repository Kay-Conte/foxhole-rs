use std::{
    collections::VecDeque,
    io::{Read, Write},
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
    http_utils::{ParseError, RequestFromBytes, ResponseToBytes},
    routing::Route, type_cache::{SharedTypeCache, TypeCache},
};

const MIN_THREADS: usize = 4;
const BUF_UNINIT_SIZE: usize = 1024;

pub struct Task {
    /// An application global type cache
    pub cache: SharedTypeCache,

    pub stream: TcpStream,
    
    /// A handle to the applications router tree
    pub router: Arc<Route>,
}

pub struct Context<'a, 'b> {
    pub global_cache: SharedTypeCache,
    pub local_cache: TypeCache,
    pub request: Request<&'b TcpStream>,
    pub path_iter: Split<'a, &'static str>,
}

struct Shared {
    /// Pool of tasks that need to be run
    pool: Mutex<VecDeque<Task>>,

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

pub struct TaskPool {
    shared: Arc<Shared>,
}

impl Default for TaskPool {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskPool {
    pub fn new() -> Self {
        let mut pool = TaskPool {
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

    /// Spawns a thread on the task pool. This function will panic on poisoned `Mutex` This will
    /// likely remain until there is a graceful shutdown mechanism
    fn spawn_thread(&mut self, should_cull: bool) {
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

                handle_connection(task);
            }
        });
    }

    /// This function can panic if the mutex is poisoned. Mutex poisoning will likely remain
    /// unhandled in the foreseeable future until a graceful shutdown mechanism is provided.
    pub fn send_task(&mut self, task: Task) {
        self.shared.pool.lock().unwrap().push_back(task);

        if self.shared.waiting_tasks.load(Ordering::Acquire) == 0 {
            self.spawn_thread(true);
        }

        self.shared.condvar.notify_one();
    }
}

fn handle_connection(mut task: Task) {
    let mut buf = vec![0; BUF_UNINIT_SIZE];
    let mut bytes_read: usize = 0;

    while let Ok(n) = task.stream.read(&mut buf[bytes_read..]) {
        if n == 0 {
            break;
        }

        bytes_read += n;

        match Request::try_headers_from_bytes(&buf[..bytes_read]) {
            Ok(req) => {
                handle_request(task, req);
                break;
            }
            Err(ParseError::Incomplete) => {
                buf.resize(buf.len() + n, 0);
                continue;
            }
            Err(_) => {
                break;
            }
        }
    }
}

fn handle_request(mut task: Task, request: Request<()>) {
    let path = request.uri().path().to_string();

    let mut path_iter = path.split("/");

    path_iter.next();

    let mut ctx = Context {
        global_cache: task.cache.clone(),
        local_cache: TypeCache::new(),
        request: request.map(|_| &task.stream),
        path_iter,
    };

    let mut cursor = task.router.as_ref();

    loop {
        for system in cursor.systems() {
            if let Some(r) = system.call(&mut ctx) {
                let _ = task.stream.write_all(&r.into_bytes());

                let _ = task.stream.flush();

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
}
