//! This module provides the application entry point.

use std::{
    net::{TcpListener, ToSocketAddrs},
    sync::{Arc, RwLock},
};

use crate::{
    routing::Route,
    tasks::{ConnectionTask, RequestTask, TaskPool},
    type_cache::TypeCache,
};

/// Application entry point. Call this to run your application.
pub fn run<A>(address: A, router: Route)
where
    A: ToSocketAddrs,
{
    run_with_cache(address, router, TypeCache::new())
}

/// Application entry point with an initialized cache.
pub fn run_with_cache<A>(address: A, router: Route, type_cache: TypeCache)
where
    A: ToSocketAddrs,
{
    let incoming = TcpListener::bind(address).expect("Could not bind to local address");

    let router = Arc::new(router);
    let type_cache = Arc::new(RwLock::new(type_cache));

    let connection_pool = TaskPool::<ConnectionTask>::default();
    let request_pool = Arc::new(TaskPool::<RequestTask>::default());

    loop {
        let Ok((stream, _addr)) = incoming.accept() else {
            continue;
        };

        let task = ConnectionTask {
            request_pool: request_pool.clone(),
            cache: type_cache.clone(),
            stream,
            router: router.clone(),
        };

        connection_pool.send_task(task);
    }
}
