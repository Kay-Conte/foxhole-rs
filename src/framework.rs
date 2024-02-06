//! This module provides the application entry point.

use std::{
    marker::PhantomData,
    net::{TcpListener, ToSocketAddrs},
    sync::{Arc, RwLock},
};

use crate::{
    connection::Connection,
    routing::Route,
    tasks::{ConnectionTask, TaskPool},
    type_cache::TypeCache,
};

/// Application entry point. Call this to run your application.
pub fn run<C>(address: impl ToSocketAddrs, router: Route)
where
    C: 'static + Connection,
{
    run_with_cache::<C>(address, router, TypeCache::new())
}

/// Application entry point with an initialized cache.
pub fn run_with_cache<C>(address: impl ToSocketAddrs, router: Route, type_cache: TypeCache)
where
    C: 'static + Connection,
{
    let incoming = TcpListener::bind(address).expect("Could not bind to local address");

    let router = Arc::new(router);
    let type_cache = Arc::new(RwLock::new(type_cache));

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
