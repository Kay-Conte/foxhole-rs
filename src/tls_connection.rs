use std::{
    io::{Read, Write},
    sync::{Arc, RwLock},
};

use mio::{event::Source, net::TcpStream};

use rustls::ServerConnection;

pub struct TlsConnection {
    pub stream: TcpStream,
    pub conn: Arc<RwLock<ServerConnection>>,
}

impl TlsConnection {
    pub fn new(stream: TcpStream, conn: Arc<RwLock<ServerConnection>>) -> Self {
        Self { stream, conn }
    }
}

impl Read for TlsConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.conn.write().unwrap().reader().read(buf)
    }
}

impl Write for TlsConnection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.conn.write().unwrap().writer().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.conn
            .write()
            .unwrap()
            .complete_io(&mut self.stream)
            .map(|_| ())
    }
}

impl Source for TlsConnection {
    fn register(
        &mut self,
        registry: &mio::Registry,
        token: mio::Token,
        interests: mio::Interest,
    ) -> std::io::Result<()> {
        self.stream.register(registry, token, interests)
    }

    fn reregister(
        &mut self,
        registry: &mio::Registry,
        token: mio::Token,
        interests: mio::Interest,
    ) -> std::io::Result<()> {
        self.stream.reregister(registry, token, interests)
    }

    fn deregister(&mut self, registry: &mio::Registry) -> std::io::Result<()> {
        self.stream.deregister(registry)
    }
}
