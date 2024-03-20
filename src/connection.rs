use std::{
    io::{BufReader, ErrorKind, Read, Write},
    net::TcpStream,
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};

use http::Request;

use crate::{
    action::RawResponse,
    get_as_slice::GetAsSlice,
    http_utils::{take_request, IntoRawBytes},
    lazy::Lazy,
    sequential_writer::{self, SequentialWriter},
    tls_connection::TlsConnection,
};

pub trait BoxedStreamMarker: Read + Write + BoxedTryClone + SetTimeout + Send + Sync {}

impl<T> BoxedStreamMarker for T where T: Read + Write + BoxedTryClone + SetTimeout + Send + Sync {}

type BoxedStream = Box<dyn BoxedStreamMarker>;

pub trait SetTimeout {
    fn set_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()>;
}

impl SetTimeout for TcpStream {
    fn set_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()> {
        self.set_read_timeout(timeout)?;
        self.set_write_timeout(timeout)
    }
}

impl SetTimeout for TlsConnection {
    fn set_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()> {
        self.stream.set_read_timeout(timeout)?;
        self.stream.set_write_timeout(timeout)
    }
}

pub trait BoxedTryClone {
    fn try_clone(&self) -> std::io::Result<BoxedStream>;
}

impl BoxedTryClone for TcpStream {
    fn try_clone(&self) -> std::io::Result<BoxedStream> {
        self.try_clone().map(|s| Box::new(s) as BoxedStream)
    }
}

impl BoxedTryClone for TlsConnection {
    fn try_clone(&self) -> std::io::Result<BoxedStream> {
        self.stream
            .try_clone()
            .map(|s| Box::new(TlsConnection::new(s, self.conn.clone())) as BoxedStream)
    }
}

pub trait Connection: Sized + Send {
    type Body: 'static + GetAsSlice + Send;
    type Responder: 'static + Responder;

    fn new(conn: BoxedStream) -> Result<Self, std::io::Error>;

    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> Result<(), std::io::Error>;

    /// Reading of the body of the previous frame may occur on subsequent calls depending on
    /// implementation
    fn next_frame(&mut self) -> Result<(Request<Self::Body>, Self::Responder), std::io::Error>;
}

pub trait Responder: Sized + Send {
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
    next_writer: Option<Receiver<BoxedStream>>,
    unfinished: Option<(usize, Sender<Vec<u8>>)>,
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
            next_writer: None,
            unfinished: None,
        })
    }

    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> Result<(), std::io::Error> {
        self.conn.set_timeout(timeout)
    }

    fn next_frame(&mut self) -> Result<(Request<Self::Body>, Self::Responder), std::io::Error> {
        if let Some((len, sender)) = self.unfinished.take() {
            let mut buf = vec![0; len];

            if self.conn.read_exact(&mut buf).is_err() {
                // Avoid blocking request tasks awaiting a body that doesn't exist.
                let _ = sender.send(vec![]);
            };

            let _ = sender.send(buf);
        }

        let req = take_request(&mut BufReader::new(&mut self.conn)).map_err(|_| {
            std::io::Error::new(ErrorKind::Other, "Failed to parse request from stream")
        })?;

        let body_len = req
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        let (lazy, sender) = Lazy::new();

        self.unfinished = Some((body_len, sender));

        Ok((req.map(|_| lazy), self.next_writer()?))
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
