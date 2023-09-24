//! This module provides the application entry point.

use std::{
    net::{TcpListener, ToSocketAddrs},
    sync::{Arc, RwLock},
};

use crate::{
    routing::Route,
    tasks::{Task, TaskPool}, type_cache::TypeCache, 
};

/// Application entry point. Call this function to run your application.
pub fn run<A>(address: A, router: Route)
where
    A: ToSocketAddrs,
{
    let incoming = TcpListener::bind(address).expect("Could not bind to local address");

    let router = Arc::new(router);
    let type_cache = Arc::new(RwLock::new(TypeCache::new()));

    let mut pool = TaskPool::new();


    loop {
        let Ok((stream, _addr)) = incoming.accept() else {
            continue;
        };

        let task = Task {
            cache: type_cache.clone(),
            stream,
            router: router.clone(),
        };

        pool.send_task(task);
    }
}
