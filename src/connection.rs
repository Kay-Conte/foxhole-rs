use std::{
    io::{ErrorKind, Read, Write},
    net::TcpStream,
    sync::mpsc::{Receiver, SyncSender},
    time::Duration,
};

use http::Request;

use crate::{
    action::RawResponse,
    get_as_slice::GetAsSlice,
    http_utils::{take_request, IntoRawBytes, ParseError},
    lazy::Lazy,
    sequential_writer::{self, SequentialWriter},
};

#[cfg(feature = "tls")]
use crate::tls_connection::TlsConnection;

/// A marker used to encapsulate the required traits for a stream used by `Http1`
pub trait BoxedStreamMarker:
    Read + Write + BoxedTryClone + SetTimeout + SetNonBlocking + Send + Sync
{
}

impl<T> BoxedStreamMarker for T where
    T: Read + Write + BoxedTryClone + SetTimeout + SetNonBlocking + Send + Sync
{
}

pub type BoxedStream = Box<dyn BoxedStreamMarker>;

pub trait SetNonBlocking {
    fn set_nonblocking(&mut self, value: bool) -> std::io::Result<()>;
}

impl SetNonBlocking for TcpStream {
    fn set_nonblocking(&mut self, value: bool) -> std::io::Result<()> {
        TcpStream::set_nonblocking(self, value)
    }
}

#[cfg(feature = "tls")]
impl SetNonBlocking for TlsConnection {
    fn set_nonblocking(&mut self, value: bool) -> std::io::Result<()> {
        self.stream.set_nonblocking(value)
    }
}

/// A trait providing a `set_timeout` function for streams
pub trait SetTimeout {
    /// Set the timeout for reads and writes on the current object
    fn set_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()>;
}

impl SetTimeout for TcpStream {
    fn set_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()> {
        self.set_read_timeout(timeout)?;
        self.set_write_timeout(timeout)
    }
}

#[cfg(feature = "tls")]
impl SetTimeout for TlsConnection {
    fn set_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()> {
        self.stream.set_read_timeout(timeout)?;
        self.stream.set_write_timeout(timeout)
    }
}

/// A trait providing a method of cloning for boxed streams
pub trait BoxedTryClone {
    fn try_clone(&self) -> std::io::Result<BoxedStream>;
}

impl BoxedTryClone for TcpStream {
    fn try_clone(&self) -> std::io::Result<BoxedStream> {
        self.try_clone().map(|s| Box::new(s) as BoxedStream)
    }
}

#[cfg(feature = "tls")]
impl BoxedTryClone for TlsConnection {
    fn try_clone(&self) -> std::io::Result<BoxedStream> {
        self.stream
            .try_clone()
            .map(|s| Box::new(TlsConnection::new(s, self.conn.clone())) as BoxedStream)
    }
}

/// A trait providing necessary functions to handle a connection
pub trait Connection: Sized + Send {
    type Body: 'static + GetAsSlice + Send;
    type Responder: 'static + Responder;

    fn new(conn: BoxedStream) -> Result<Self, std::io::Error>;

    fn set_timeout(&mut self, timeout: Option<Duration>) -> Result<(), std::io::Error>;

    fn set_nonblocking(&mut self, value: bool) -> std::io::Result<()>;

    /// Reading of the body of the previous frame may occur on subsequent calls depending on
    /// implementation
    fn next_frame(&mut self) -> Result<(Request<Self::Body>, Self::Responder), std::io::Error>;

    fn upgrade(self) -> BoxedStream;
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
    conn: BoxedStream,
    buf: Vec<u8>,
    read: usize,
    next_writer: Option<Receiver<BoxedStream>>,
    unfinished: Option<(usize, SyncSender<Vec<u8>>)>,
}

impl Http1 {
    fn next_writer(&mut self) -> Result<SequentialWriter<BoxedStream>, std::io::Error> {
        Ok(match self.next_writer.take() {
            Some(writer) => {
                let (writer, receiver) =
                    SequentialWriter::new(sequential_writer::State::Waiting(writer));

                self.next_writer = Some(receiver);

                writer
            }

            None => {
                let (writer, receiver) =
                    SequentialWriter::new(sequential_writer::State::Writer(self.conn.try_clone()?));

                self.next_writer = Some(receiver);

                writer
            }
        })
    }
}

impl Connection for Http1 {
    type Body = Lazy<Vec<u8>>;
    type Responder = SequentialWriter<BoxedStream>;

    fn new(conn: BoxedStream) -> Result<Self, std::io::Error> {
        Ok(Self {
            conn,
            buf: vec![0; 1024],
            read: 0,
            next_writer: None,
            unfinished: None,
        })
    }

    fn set_timeout(&mut self, timeout: Option<Duration>) -> Result<(), std::io::Error> {
        self.conn.set_timeout(timeout)
    }

    fn set_nonblocking(&mut self, value: bool) -> std::io::Result<()> {
        self.conn.set_nonblocking(value)
    }

    fn next_frame(&mut self) -> Result<(Request<Self::Body>, Self::Responder), std::io::Error> {
        if let Some((body_idx, sender)) = &self.unfinished {
            if self.read != self.buf.len() {
                loop {
                    let n = self.conn.read(&mut self.buf[self.read..])?;
                    self.read += n;
                }
            }

            let body = self.buf[*body_idx..].to_vec();

            let _ = sender.send(body);

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

    fn upgrade(self) -> BoxedStream {
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
