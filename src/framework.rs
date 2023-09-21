use std::{
    net::{TcpListener, ToSocketAddrs},
    sync::Arc,
};

use crate::{
    routing::Route,
    tasks::{Task, TaskPool},
};

pub fn run<A>(address: A, router: Route)
where
    A: ToSocketAddrs,
{
    let incoming = TcpListener::bind(address).expect("Could not bind to local address");

    let router = Arc::new(router);

    let mut pool = TaskPool::new();

    loop {
        let Ok((stream, _addr)) = incoming.accept() else {
            continue;
        };

        let task = Task {
            stream,
            router: router.clone(),
        };

        pool.send_task(task);
    }
}
