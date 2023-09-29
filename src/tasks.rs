use std::{
    collections::VecDeque,
    net::TcpStream,
    str::Split,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Condvar, Mutex,
    },
    time::Duration, io::{BufReader, BufRead},
};

use http::Request;

use crate::{
    http_utils::{ParseError, RequestFromBytes, ResponseToBytes},
    routing::Route,
    type_cache::{TypeCache, TypeCacheShared},
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
    pub router: Arc<Route>
}

pub struct Context<'a> {
    pub global_cache: TypeCacheShared,
    pub local_cache: TypeCache,
    pub request: Request<()>,
    pub path_iter: Split<'a, &'static str>,
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

    let mut path_iter = path.split("/");

    path_iter.next();

    let mut ctx = Context {
        global_cache: task.cache.clone(),
        local_cache: TypeCache::new(),
        request: task.request,
        path_iter,
    };

    let mut cursor = task.router.as_ref();

    loop {
        for system in cursor.systems() {
            if let Some(r) = system.call(&mut ctx) {
                let _ = task.writer.send(&r.into_bytes());

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

    let _ = task.writer.send(&404u16.maybe_response().unwrap().into_bytes());
}

/// # Panics
/// Panics if the stream is closed early. This will be fixed.
fn handle_connection(task: ConnectionTask) {
    task.stream
        .set_read_timeout(Some(Duration::from_secs(TIMEOUT)))
        .expect("Shouldn't fail unless duration is 0");

    // FIXME panics if stream closed early
    let mut writer = SequentialWriter::new(sequential_writer::State::Writer(task.stream.try_clone().unwrap()));
    let mut reader = BufReader::new(task.stream).lines();

    loop {
        match Request::take_request(&mut reader) {
            Ok(req) => {
                task.request_pool.send_task(RequestTask {
                    cache: task.cache.clone(),
                    request: req,
                    writer: writer.0,
                    router: task.router.clone(),
                });

                writer = SequentialWriter::new(sequential_writer::State::Waiting(writer.1));

                // FIXME multiplexing is actually slower than closing the connection every time. I
                // think this may be something to do with incomplete http parsing.
                return;
            }
            Err(ParseError::Incomplete) => {
                continue;
            }
            Err(_) => {
                return;
            }
        }
    }
}
