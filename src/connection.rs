use std::{
    io::{ErrorKind, Read, Write},
    sync::{
        mpsc::{Receiver, SyncSender},
        Arc, Mutex,
    },
};

use http::Request;
use mio::event::Source;

use crate::{
    action::RawResponse,
    get_as_slice::GetAsSlice,
    http_utils::{take_request, IntoRawBytes, ParseError},
    lazy::Lazy,
    sequential_writer::{self, SequentialWriter},
};

/// A marker used to encapsulate the required traits for a stream used by `Http1`
pub trait BoxedStreamMarker: Read + Write + Source + Send + Sync {}

impl<T> BoxedStreamMarker for T where T: Read + Write + Source + Send + Sync {}

#[derive(Clone)]
pub struct SharedStream(Arc<Mutex<dyn BoxedStreamMarker>>);

impl Read for SharedStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().read(buf)
    }
}

impl Write for SharedStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

/// A trait providing necessary functions to handle a connection
pub trait Connection: Source + Sized + Send {
    type Body: 'static + GetAsSlice + Send;
    type Responder: 'static + Responder;

    fn new(conn: Box<dyn BoxedStreamMarker>) -> Result<Self, std::io::Error>;

    /// Reading of the body of the previous frame may occur on subsequent calls depending on
    /// implementation
    fn poll(&mut self) -> Result<(Request<Self::Body>, Self::Responder), std::io::Error>;

    fn upgrade(self) -> SharedStream;
}

/// A trait providing necessary functionality to respond to a connection
pub trait Responder: Sized + Send {
    /// Write bytes of response to the underlying writer. This can be expected to be the full
    /// response
    fn write_bytes(self, bytes: Vec<u8>) -> Result<(), std::io::Error>;

    fn respond(self, response: impl Into<RawResponse>) -> Result<(), std::io::Error> {
        let response: RawResponse = response.into();

        let bytes = response.into_raw_bytes();

        self.write_bytes(bytes)
    }
}

/// HTTP 1.1
pub struct Http1 {
    conn: SharedStream,
    buf: Vec<u8>,
    read: usize,
    next_writer: Option<Receiver<SharedStream>>,
    unfinished: Option<(usize, SyncSender<Vec<u8>>)>,
}

impl Http1 {
    fn next_writer(&mut self) -> Result<SequentialWriter<SharedStream>, std::io::Error> {
        Ok(match self.next_writer.take() {
            Some(writer) => {
                let (writer, receiver) =
                    SequentialWriter::new(sequential_writer::State::Waiting(writer));

                self.next_writer = Some(receiver);

                writer
            }

            None => {
                let (writer, receiver) =
                    SequentialWriter::new(sequential_writer::State::Writer(self.conn.clone()));

                self.next_writer = Some(receiver);

                writer
            }
        })
    }
}

impl Source for Http1 {
    fn register(
        &mut self,
        registry: &mio::Registry,
        token: mio::Token,
        interests: mio::Interest,
    ) -> std::io::Result<()> {
        self.conn
            .0
            .lock()
            .unwrap()
            .register(registry, token, interests)
    }

    fn reregister(
        &mut self,
        registry: &mio::Registry,
        token: mio::Token,
        interests: mio::Interest,
    ) -> std::io::Result<()> {
        self.conn
            .0
            .lock()
            .unwrap()
            .reregister(registry, token, interests)
    }

    fn deregister(&mut self, registry: &mio::Registry) -> std::io::Result<()> {
        self.conn.0.lock().unwrap().deregister(registry)
    }
}

impl Connection for Http1 {
    type Body = Lazy<Vec<u8>>;
    type Responder = SequentialWriter<SharedStream>;

    fn new(conn: Box<dyn BoxedStreamMarker>) -> Result<Self, std::io::Error> {
        Ok(Self {
            conn: SharedStream(Arc::new(Mutex::new(conn))),
            buf: vec![0; 1024],
            read: 0,
            next_writer: None,
            unfinished: None,
        })
    }

    fn poll(&mut self) -> Result<(Request<Self::Body>, Self::Responder), std::io::Error> {
        if let Some((body_idx, sender)) = &self.unfinished {
            if self.read != self.buf.len() {
                loop {
                    let n = self.conn.read(&mut self.buf[self.read..])?;
                    self.read += n;
                }
            }

            let body = self.buf[*body_idx..].to_vec();

            sender.send(body).expect("Failed to send body to Lazy");

            self.buf.resize(1024, 0);
            self.read = 0;
            self.unfinished = None;
        }

        let n = self.conn.read(&mut self.buf[self.read..])?;
        self.read += n;

        let (req, body_idx) = take_request(&self.buf[..self.read]).map_err(|e| match e {
            ParseError::Unfinished => {
                std::io::Error::new(ErrorKind::WouldBlock, "Unfinished request")
            }
            _ => std::io::Error::new(ErrorKind::Other, "Failed to parse request from stream"),
        })?;

        let body_len = req
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        let (lazy, sender) = Lazy::new();

        if body_len == 0 {
            sender
                .send(Vec::new())
                .expect("Failed to send body to Lazy");

            self.buf.clear();
            self.buf.resize(1024, 0);
            self.read = 0;
        } else {
            self.buf.resize(body_idx + body_len, 0);
            self.unfinished = Some((body_idx, sender));
        }

        Ok((req.map(|_| lazy), self.next_writer()?))
    }

    fn upgrade(self) -> SharedStream {
        self.conn
    }
}

impl<W> Responder for SequentialWriter<W>
where
    W: Write + Send + Sync,
{
    fn write_bytes(self, bytes: Vec<u8>) -> Result<(), std::io::Error> {
        self.send(&bytes)
    }
}
