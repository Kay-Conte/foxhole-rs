use std::{
    io::{BufReader, ErrorKind, Write, Read},
    net::TcpStream,
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};

use http::Request;

use crate::{
    action::RawResponse,
    http_utils::{take_request, IntoRawBytes},
    lazy::Lazy,
    sequential_writer::{self, SequentialWriter},
};

pub trait Connection: Sized {
    type Frame;

    fn new(conn: TcpStream) -> Result<Self, std::io::Error>;

    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> Result<(), std::io::Error>;

    /// Reading of the body of the previous frame may occur on subsequent calls depending on
    /// implementation
    fn next_frame(&mut self) -> Result<Self::Frame, std::io::Error>;

    fn write_bytes(&mut self, bytes: Vec<u8>) -> Result<(), std::io::Error>;

    fn respond(&mut self, response: impl Into<RawResponse>) -> Result<(), std::io::Error> {
        let response: RawResponse = response.into();

        let bytes = response.into_raw_bytes();

        self.write_bytes(bytes)
    }
}

/// HTTP 1.1
pub struct Http1 {
    conn: TcpStream,
    next_writer: Option<Receiver<TcpStream>>,
    unfinished: Option<(usize, Sender<Vec<u8>>)>,
}

impl Http1 {
    fn next_writer(&mut self) -> Result<SequentialWriter<TcpStream>, std::io::Error> {
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
    type Frame = (Request<Lazy<Vec<u8>>>, SequentialWriter<TcpStream>);

    fn new(conn: TcpStream) -> Result<Self, std::io::Error> {
        Ok(Self {
            conn,
            next_writer: None,
            unfinished: None,
        })
    }

    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> Result<(), std::io::Error> {
        self.conn.set_read_timeout(timeout)
    }

    fn next_frame(&mut self) -> Result<Self::Frame, std::io::Error> {
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

    fn write_bytes(&mut self, bytes: Vec<u8>) -> Result<(), std::io::Error> {
        self.conn.write_all(&bytes)
    }
}
