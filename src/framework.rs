//! This module provides the application entry point.

use std::{
    net::{TcpListener, ToSocketAddrs},
    sync::{Arc, RwLock},
};

use crate::{
    routing::Route,
    tasks::{ConnectionTask, TaskPool, RequestTask}, type_cache::TypeCache, 
};

/// Application entry point. Call this function to run your application.
pub fn run<A>(address: A, router: Route)
where
    A: ToSocketAddrs,
{
    let incoming = TcpListener::bind(address).expect("Could not bind to local address");

    let router = Arc::new(router);
    let type_cache = Arc::new(RwLock::new(TypeCache::new()));

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
