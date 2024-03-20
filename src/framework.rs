//! This module provides the application entry point.

pub use framework::*;

#[cfg(not(feature = "tls"))]
mod framework {
    use std::{
        marker::PhantomData,
        net::{TcpListener, ToSocketAddrs},
        sync::{Arc, RwLock},
    };

    use crate::{
        connection::Connection,
        routing::{Router, Scope},
        tasks::{ConnectionTask, TaskPool},
        type_cache::TypeCache,
        Request, Response,
        layers::Layer,
    };

    pub struct App {
        router: Router,
        type_cache: TypeCache,
    }
    
    impl App {
        pub fn builder(scope: impl Into<Scope>) -> Self {
            Self {
                router: Router::new(scope),
                type_cache: TypeCache::new(),
            }
        }

        pub fn request_layer(mut self, layer: impl 'static + Layer<Request> + Send + Sync) -> Self {
            self.router.request_layer = Box::new(layer);
            self
        }

        pub fn response_layer(mut self, layer: impl 'static + Layer<Response> + Send + Sync) -> Self {
            self.router.response_layer = Box::new(layer);
            self
        }

        pub fn cache(mut self, cache: TypeCache) -> Self {
            self.type_cache = cache; 
            self
        }

        pub fn run<C>(self, address: impl ToSocketAddrs)
        where
            C: 'static + Connection,
        {
            let incoming = TcpListener::bind(address).expect("Could not bind to local address");

            let router = Arc::new(self.router);
            let type_cache = Arc::new(RwLock::new(self.type_cache));

            let task_pool = TaskPool::new();

            loop {
                let Ok((stream, _addr)) = incoming.accept() else {
                    continue;
                };

                let task = ConnectionTask::<C> {
                    task_pool: task_pool.clone(),
                    cache: type_cache.clone(),
                    stream,
                    router: router.clone(),
                    phantom_data: PhantomData,
                };

                task_pool.send_task(task);
            }
        }
    }
}

#[cfg(feature = "tls")]
mod framework {
    use std::{
        marker::PhantomData,
        net::{TcpListener, ToSocketAddrs},
        sync::{Arc, RwLock},
    };

    use rustls::ServerConfig;

    use crate::{
        connection::Connection,
        routing::Router,
        tasks::{SecuredConnectionTask, TaskPool},
        type_cache::TypeCache,
    };

    /// Application entry point. Call this to run your application.
    pub fn run<C>(address: impl ToSocketAddrs, router: Router, tls_config: ServerConfig)
    where
        C: 'static + Connection,
    {
        run_with_cache::<C>(address, router, TypeCache::new(), tls_config)
    }

    /// Application entry point with an initialized cache.
    pub fn run_with_cache<C>(
        address: impl ToSocketAddrs,
        router: Router,
        type_cache: TypeCache,
        tls_config: ServerConfig,
    ) where
        C: 'static + Connection,
    {
        let incoming = TcpListener::bind(address).expect("Could not bind to local address");

        let router = Arc::new(router);
        let type_cache = Arc::new(RwLock::new(type_cache));

        let task_pool = TaskPool::new();

        let tls_config = Arc::new(tls_config);

        loop {
            let Ok((stream, _addr)) = incoming.accept() else {
                continue;
            };

            let task = SecuredConnectionTask::<C> {
                task_pool: task_pool.clone(),
                cache: type_cache.clone(),
                stream,
                router: router.clone(),
                tls_config: tls_config.clone(),
                phantom_data: PhantomData,
            };

            task_pool.send_task(task);
        }
    }
}
