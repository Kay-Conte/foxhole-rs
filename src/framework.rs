//! This module provides the application entry point.
use std::{
    collections::HashMap,
    io,
    sync::{mpsc::Receiver, Arc, RwLock},
};

use crate::{
    channel::{sync_channel, SyncSender},
    connection::Connection,
    layers::{BoxLayer, DefaultResponseGroup, Layer},
    routing::Router,
    tasks::{handle_connection, ConnectionContext, TaskPool},
    type_cache::TypeCache,
    Request, Response,
};

use mio::{net::TcpListener, Events, Interest, Poll, Token, Waker};

#[cfg(feature = "tls")]
use crate::tls_connection::TlsConnection;

#[cfg(feature = "tls")]
use rustls::ServerConfig;

#[cfg(feature = "tls")]
use rustls::ServerConnection;

const WAKE: Token = Token(0);
const INCOMING: Token = Token(1);

/// Main application entry point. Construct this type to run your application.
pub struct App {
    pub(crate) router: Router,
    pub(crate) request_layer: BoxLayer<Request>,
    pub(crate) response_layer: BoxLayer<Response>,
    pub(crate) cache: Arc<TypeCache>,

    #[cfg(feature = "tls")]
    pub(crate) tls_config: Option<Arc<ServerConfig>>,
}

impl App {
    /// Constructs a new application
    pub fn builder(scope: Router) -> Self {
        Self {
            router: scope.into(),
            request_layer: Box::new(()),
            response_layer: Box::new(DefaultResponseGroup::new()),
            cache: Arc::new(TypeCache::new()),

            #[cfg(feature = "tls")]
            tls_config: None,
        }
    }

    /// Overrides the default request `Layer` if one is set
    pub fn request_layer(mut self, layer: impl 'static + Layer<Request> + Send + Sync) -> Self {
        self.request_layer = Box::new(layer);
        self
    }

    /// Overrides the default response `Layer` if one is set
    pub fn response_layer(mut self, layer: impl 'static + Layer<Response> + Send + Sync) -> Self {
        self.response_layer = Box::new(layer);
        self
    }

    /// Sets the cache to be used by the application
    pub fn cache(mut self, cache: TypeCache) -> Self {
        self.cache = Arc::new(cache);
        self
    }

    #[cfg(feature = "tls")]
    pub fn tls_config(mut self, tls_config: ServerConfig) -> Self {
        self.tls_config = Some(Arc::new(tls_config));
        self
    }

    /// Executes the application. This will currently never return.
    pub fn run<C>(self, address: &str) -> Result<(), Box<dyn std::error::Error>>
    where
        C: 'static + Connection,
    {
        #[cfg(feature = "tls")]
        let Some(tls_config) = self.tls_config.clone() else {
            return Err("Missing Tls Config".into());
        };

        let mut incoming = TcpListener::bind(address.parse()?)?;

        let shared = Arc::new(self);
        let task_pool = Arc::new(TaskPool::new());

        let mut poll = Poll::new()?;
        let mut events = Events::with_capacity(1024);

        poll.registry()
            .register(&mut incoming, INCOMING, Interest::READABLE)?;

        let mut next_token = INCOMING.0 + 1;

        let waker = Arc::new(Waker::new(poll.registry(), WAKE)?);

        let mut context_map: HashMap<Token, ConnectionContext<C>> = HashMap::new();
        let (sender, receiver): (SyncSender<_>, Receiver<ConnectionContext<C>>) =
            sync_channel(waker.clone(), 1024);

        loop {
            match poll.poll(&mut events, None) {
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => {
                    println!("{e:?}");
                    break;
                }
            };

            for event in events.iter() {
                match event.token() {
                    WAKE => {
                        while let Ok(mut task) = receiver.try_recv() {
                            let Some(conn) = &mut task.conn else {
                                continue;
                            };

                            poll.registry()
                                .register(conn, task.token, Interest::READABLE)?;

                            context_map.insert(task.token.clone(), task);
                        }
                    }
                    INCOMING => {
                        while let Ok((mut stream, _addr)) = incoming.accept() {
                            #[cfg(feature = "tls")]
                            let stream = {
                                let mut connection = ServerConnection::new(tls_config.clone())?;

                                connection.complete_io(&mut stream)?;

                                TlsConnection::new(stream, Arc::new(RwLock::new(connection)))
                            };

                            let mut conn = C::new(Box::new(stream))?;

                            let token = Token(next_token);
                            next_token += 1;

                            poll.registry().register(
                                &mut conn,
                                token,
                                Interest::READABLE.add(Interest::WRITABLE),
                            )?;

                            context_map.insert(
                                token,
                                ConnectionContext {
                                    token,
                                    task_pool: task_pool.clone(),
                                    app: shared.clone(),
                                    conn: Some(conn),
                                    register: None,
                                },
                            );

                            next_token += 1;
                        }
                    }
                    token => {
                        if !event.is_readable() {
                            continue;
                        }

                        let mut ctx = context_map.remove(&token).unwrap();

                        let Some(conn) = &mut ctx.conn else {
                            continue;
                        };

                        poll.registry().deregister(conn)?;
                        ctx.register = Some(sender.clone());

                        task_pool.send_task(Box::new(move || handle_connection(ctx)))
                    }
                }
            }
        }

        Ok(())
    }
}
