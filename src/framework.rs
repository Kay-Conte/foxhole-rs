//! This module provides the application entry point.
//!
use std::{
    marker::PhantomData,
    net::{TcpListener, ToSocketAddrs},
    sync::{Arc, RwLock},
};

use crate::{
    connection::Connection,
    layers::{BoxLayer, DefaultResponseGroup, Layer},
    routing::Scope,
    tasks::TaskPool,
    type_cache::TypeCache,
    Request, Response,
};

#[cfg(not(feature = "tls"))]
use crate::tasks::ConnectionTask;

#[cfg(feature = "tls")]
use crate::tasks::SecuredConnectionTask;

#[cfg(feature = "tls")]
use rustls::ServerConfig;

pub struct App {
    tree: Scope,
    request_layer: BoxLayer<Request>,
    response_layer: BoxLayer<Response>,
    type_cache: TypeCache,

    #[cfg(feature = "tls")]
    tls_config: Option<ServerConfig>,
}

impl App {
    pub fn builder(scope: impl Into<Scope>) -> Self {
        Self {
            tree: scope.into(),
            request_layer: Box::new(()),
            response_layer: Box::new(DefaultResponseGroup::new()),
            type_cache: TypeCache::new(),

            #[cfg(feature = "tls")]
            tls_config: None,
        }
    }

    pub fn request_layer(mut self, layer: impl 'static + Layer<Request> + Send + Sync) -> Self {
        self.request_layer = Box::new(layer);
        self
    }

    pub fn response_layer(mut self, layer: impl 'static + Layer<Response> + Send + Sync) -> Self {
        self.response_layer = Box::new(layer);
        self
    }

    pub fn cache(mut self, cache: TypeCache) -> Self {
        self.type_cache = cache;
        self
    }

    #[cfg(feature = "tls")]
    pub fn tls_config(mut self, tls_config: ServerConfig) -> Self {
        self.tls_config = Some(tls_config);
        self
    }

    pub fn run<C>(self, address: impl ToSocketAddrs)
    where
        C: 'static + Connection,
    {
        let incoming = TcpListener::bind(address).expect("Could not bind to local address");

        let type_cache = Arc::new(RwLock::new(self.type_cache));
        let router = Arc::new(self.tree);
        let request_layer = Arc::new(self.request_layer);
        let response_layer = Arc::new(self.response_layer);

        #[cfg(feature = "tls")]
        let tls_config = match self.tls_config {
            Some(conf) => Arc::new(conf),
            None => panic!("Missing tls config"), // change message prob
        };

        let task_pool = TaskPool::new();

        loop {
            let Ok((stream, _addr)) = incoming.accept() else {
                continue;
            };

            #[cfg(feature = "tls")]
            let task = SecuredConnectionTask::<C> {
                task_pool: task_pool.clone(),
                cache: type_cache.clone(),
                stream,
                router: router.clone(),
                tls_config: tls_config.clone(),
                phantom_data: PhantomData,
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
