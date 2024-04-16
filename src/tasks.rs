use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Condvar, Mutex,
    },
    time::Duration,
};

use crate::{
    channel,
    connection::{Connection, Responder},
    get_as_slice::GetAsSlice,
    layers::BoxLayer,
    url_decoding, Action, App, IntoResponse, Request, Response, Router, TypeCache,
};
use mio::Token;

const MIN_THREADS: usize = 4;

/// Request with a boxed body implementing `GetAsSlice` This is the standard request type
/// throughout the library
pub type BoxedBodyRequest = http::Request<Box<dyn 'static + GetAsSlice + Send>>;

pub(crate) struct ConnectionContext<C> {
    pub app: Arc<App>,
    pub task_pool: Arc<TaskPool>,
    pub token: Token,
    pub conn: C,
    pub register: Option<channel::SyncSender<ConnectionContext<C>>>,
}

pub(crate) fn handle_connection<C>(mut ctx: ConnectionContext<C>)
where
    C: Connection,
{
    let Ok((req, res)) = ctx.conn.poll() else {
        return;
    };

    let req: Request = req.map(|i| {
        let b: Box<dyn 'static + GetAsSlice + Send> = Box::new(i);

        b
    });

    let req_ctx = RequestContext {
        app: ctx.app.clone(),
        request: req,
        responder: res,
        upgrade: None,
    };

    ctx.task_pool
        .send_task(Box::new(move || handle_request(req_ctx)));

    if let Some(mut register) = ctx.register.clone() {
        register.send(ctx).unwrap();
    }
}

pub(crate) struct RequestContext<R> {
    pub app: Arc<App>,
    pub request: BoxedBodyRequest,
    pub responder: R,
    pub upgrade: Option<()>,
}

fn respond_fallback<R>(
    ctx: RequestState,
    router: &Router,
    response_layer: &BoxLayer<Response>,
    responder: R,
) -> std::io::Result<()>
where
    R: Responder,
{
    let action = router.get_fallback().call(&ctx, VecDeque::new());

    let mut res = match action {
        Action::Respond(r) => r,
        _ => 500u16.response(),
    };

    response_layer.execute(&mut res);

    let _ = responder.respond(res);

    Ok(())
}

pub(crate) fn handle_request<R>(mut ctx: RequestContext<R>)
where
    R: Responder,
{
    ctx.app.request_layer.execute(&mut ctx.request);

    let path = ctx.request.uri().to_string();

    let (path, query) = path.split_once("?").unwrap_or((&path, ""));

    let Some(query) = url_decoding::map(query) else {
        let _ = ctx.responder.respond(400u16.response());

        return;
    };

    let state = RequestState {
        cache: ctx.app.cache.clone(),
        request: ctx.request,
        query,
    };

    let Some((handler, captures)) = ctx.app.router.route(path) else {
        let _ = respond_fallback(
            state,
            &ctx.app.router,
            &ctx.app.response_layer,
            ctx.responder,
        );

        return;
    };

    let Some(system) = handler.get(state.request.method()) else {
        let _ = respond_fallback(
            state,
            &ctx.app.router,
            &ctx.app.response_layer,
            ctx.responder,
        );

        return;
    };

    let action = system.call(&state, captures);

    match action {
        Action::Respond(mut r) => {
            ctx.app.response_layer.execute(&mut r);

            let _ = ctx.responder.respond(r);

            return;
        }
        #[cfg(feature = "websocket")]
        Action::Upgrade(r, f) => {
            // Todo move this handling to the constructor of `Action::Upgrade`
            todo!();
            // let Some(connection) = ctx.upgrade else {
            //     return;
            // };
            //
            // let response = r(&ctx.request);
            // let _ = ctx.responder.respond(response);
            //
            // f(connection.upgrade());
            //
            // return;
        }
        Action::None => {
            let _ = respond_fallback(
                state,
                &ctx.app.router,
                &ctx.app.response_layer,
                ctx.responder,
            );

            return;
        }
    }
}

/// Holds the state of the request handling. This can be accessed via the `Resolve` trait.
pub struct RequestState {
    pub cache: Arc<TypeCache>,
    pub request: BoxedBodyRequest,
    pub query: HashMap<String, String>,
}

struct Registration<'a> {
    nb: &'a AtomicUsize,
}

impl<'a> Registration<'a> {
    fn new(nb: &'a AtomicUsize) -> Registration<'a> {
        nb.fetch_add(1, Ordering::Release);
        Registration { nb }
    }
}

impl<'a> Drop for Registration<'a> {
    fn drop(&mut self) {
        self.nb.fetch_sub(1, Ordering::Release);
    }
}

pub struct Shared {
    /// Pool of tasks that need to be run
    pool: Mutex<VecDeque<Box<dyn FnOnce() + Send + 'static>>>,

    /// Conditional var used to sleep and wake threads
    pub condvar: Condvar,

    /// Total number of threads currently waiting for a task
    waiting_tasks: AtomicUsize,

    active_tasks: AtomicUsize,
}

#[derive(Clone)]
pub(crate) struct TaskPool {
    pub shared: Arc<Shared>,
}

impl TaskPool {
    pub fn new() -> Self {
        let pool = TaskPool {
            shared: Arc::new(Shared {
                pool: Mutex::new(VecDeque::new()),
                condvar: Condvar::new(),
                waiting_tasks: AtomicUsize::new(0),
                active_tasks: AtomicUsize::new(0),
            }),
        };

        for _ in 0..MIN_THREADS {
            pool.spawn_thread(None);
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
    fn spawn_thread(&self, initial_task: Option<Box<dyn FnOnce() + 'static + Send>>) {
        let shared = self.shared.clone();

        std::thread::spawn(move || {
            let shared = shared;
            let _active = Registration::new(&shared.active_tasks);

            if let Some(t) = initial_task {
                t();
            }

            loop {
                let mut pool = shared.pool.lock().unwrap();

                let task = loop {
                    if let Some(task) = pool.pop_front() {
                        break task;
                    }

                    let _waiting = Registration::new(&shared.waiting_tasks);

                    if shared.active_tasks.load(Ordering::Acquire) <= MIN_THREADS {
                        pool = shared.condvar.wait(pool).unwrap();
                    } else {
                        let (new_lock, waitres) = shared
                            .condvar
                            .wait_timeout(pool, Duration::from_secs(5))
                            .unwrap();

                        pool = new_lock;

                        if waitres.timed_out() && pool.is_empty() {
                            return;
                        }
                    }
                };

                drop(pool);

                task();
            }
        });
    }

    /// Adds a task to the task pool and spawns a thread if there is none available
    ///
    /// # Panics
    /// This function can panic if the mutex is poisoned. Mutex poisoning will likely remain
    /// unhandled in the foreseeable future until a graceful shutdown mechanism is provided.
    pub fn send_task<T>(&self, task: T)
    where
        T: FnOnce() + 'static + Send,
    {
        let mut queue = self.shared.pool.lock().unwrap();

        if self.shared.waiting_tasks.load(Ordering::Acquire) == 0 {
            self.spawn_thread(Some(Box::new(task)));
        } else {
            queue.push_back(Box::new(task));
            self.shared.condvar.notify_one();
        }
    }
}
