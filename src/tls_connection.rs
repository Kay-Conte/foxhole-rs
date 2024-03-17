use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::{Arc, RwLock},
};

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
