use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    str::Split,
    thread,
};

use http::Request;

use crate::{http_utils::{RequestExt, ResponseExt}, routing::Node, systems::RawResponse};

pub struct Settings<A>
where
    A: ToSocketAddrs,
{
    address: A,
    router: Node,
}

struct InternalContext<'a> {
    stream: TcpStream,
    router: &'a Node,
}

pub struct Context<'a, 'b> {
    internal: InternalContext<'a>,
    pub request: Request<String>,
    pub path_iter: Split<'b, &'static str>,
}

pub fn run<A>(address: A, root: Node)
where
    A: ToSocketAddrs,
{
    let incoming = TcpListener::bind(address).expect("Could not bind to local addrss");

    thread::scope(|s| loop {
        let Ok((stream, _addr)) = incoming.accept() else {
            continue;
        };

        let ctx = InternalContext {
            stream,
            router: &root,
        };

        s.spawn(|| handle_connection(ctx));
    });
}

fn handle_connection(mut ctx: InternalContext) {
    let mut buf = vec![0; 1024];
    let mut bytes_read: usize = 0;

    // Some left behind fragments of content-length aware reading.
    let res = ctx.stream.read(&mut buf[bytes_read..]);

    match res {
        Ok(n) => {
            if n == 0 {
                return;
            }

            bytes_read += n;

            let request = Request::parse_request(&buf[..bytes_read]).expect("Failed to parse request");

            handle_request(ctx, request);
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => return,
        Err(_) => return,
    }
}

fn handle_request(ctx: InternalContext, request: Request<String>) {
    let path = request.uri().path().to_string();

    let mut path_iter = path.split("/");

    // Discard first in iter as it will always be an empty string
    path_iter.next();

    let mut ctx = Context {
        internal: ctx,
        request,
        path_iter,
    };

    let mut cursor = ctx.internal.router;

    loop {
        for system in cursor.systems() {
            if let Some(r) = system.call(&mut ctx) {
                respond(ctx.internal.stream, r);
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
}

fn respond(mut stream: TcpStream, response: RawResponse) {
    let _ = stream.write_all(&response.to_bytes());

    let _ = stream.flush();
}
