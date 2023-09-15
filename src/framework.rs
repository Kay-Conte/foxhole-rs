use std::{net::{ToSocketAddrs, TcpListener, TcpStream}, io::Read, thread};

use httparse::Request;

use crate::routing::Node;

const SUPPORTED_HEADER_COUNT: usize = 16; 

pub struct Context { }

pub fn run<A>(address: A, root: Node) where A: ToSocketAddrs {
    let incoming = TcpListener::bind(address).expect("Could not bind to local addrss");

    thread::scope(|s| {
        loop {
            let Ok((stream, _addr)) = incoming.accept() else { continue };
            
            s.spawn(|| handle_connection(stream, &root));
        }
    });
}

fn handle_connection(mut stream: TcpStream, root: &Node) {
    let mut buf = vec![0; 1024];
    let mut bytes_read: usize = 0;

    loop {
        let res = stream.read(&mut buf[bytes_read..]);

        match res {
            Ok(n) => {
                if n == 0 {
                    return;
                }

                bytes_read += n;

                let mut headers = [httparse::EMPTY_HEADER; SUPPORTED_HEADER_COUNT];
                let mut req = httparse::Request::new(&mut headers);

                match req.parse(&buf[..bytes_read]) {
                    Ok(httparse::Status::Complete(size)) => {
                        handle_request(stream, root, &req);

                        break;
                    },
                    Ok(httparse::Status::Partial) => {
                        continue;
                    }
                    Err(_) => todo!(),
                }
            },
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(_) => return,
        }
    }
}

fn handle_request(mut stream: TcpStream, root: &Node, req: &Request) {
    let url_parts = req.path.expect("Request has no path").split("/");
}
