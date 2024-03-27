use std::{io::ErrorKind, time::Duration};

use base64::{engine::general_purpose::STANDARD, Engine};
use sha1::{Digest, Sha1};

use crate::{
    action::IntoAction, connection::BoxedStream, Action, IntoResponse, Request, Resolve,
    ResolveGuard, Response,
};

const GUID: &'static str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

pub(crate) fn encode_key(key: &str) -> String {
    let concatenated = format!("{}{}", key, GUID);

    let mut hasher = Sha1::new();
    hasher.update(concatenated);
    let result = hasher.finalize();

    STANDARD.encode(&result)
}

fn respond_handshake(req: &Request) -> Response {
    let supports_version = req
        .headers()
        .get("sec-websocket-version")
        .and_then(|i| i.to_str().ok())
        .and_then(|i| i.split(",").find(|i| i.trim() == "13"))
        .is_some();

    if !supports_version {
        return http::Response::builder()
            .status(426)
            .header("sec-websocket-version", "13")
            .body(Vec::new())
            .unwrap();
    }

    let Some(key) = req
        .headers()
        .get("sec-websocket-key")
        .and_then(|i| i.to_str().ok())
    else {
        return 400u16.response();
    };

    let accept = encode_key(key);

    http::Response::builder()
        .status(101)
        .header("sec-websocket-accept", accept)
        .header("upgrade", "websocket")
        .header("connection", "upgrade")
        .body(Vec::new())
        .unwrap()
}

/// Guards on the `connection: upgrade` header. Provides a method of constructing `Websocket`
/// return value
pub struct Upgrade;

impl Upgrade {
    /// Convert this type to a `Websocket`. Return the `Websocket` from a system to handle the connection
    pub fn handle(&self, f: fn(WebsocketConnection)) -> Websocket {
        Websocket(f)
    }
}

impl<'a> Resolve<'a> for Upgrade {
    type Output = Upgrade;

    fn resolve(
        ctx: &'a crate::RequestState,
        _path_iter: &mut crate::PathIter,
    ) -> crate::ResolveGuard<Self::Output> {
        let Some(header) = ctx
            .request
            .headers()
            .get("connection")
            .and_then(|i| i.to_str().ok())
        else {
            return ResolveGuard::None;
        };

        if header.to_lowercase() == "upgrade" {
            ResolveGuard::Value(Upgrade)
        } else {
            ResolveGuard::None
        }
    }
}

pub struct Websocket(fn(WebsocketConnection));

impl IntoAction for Websocket {
    fn action(self) -> crate::Action {
        Action::Upgrade(
            respond_handshake,
            Box::new(move |stream| (self.0)(WebsocketConnection::new(stream))),
        )
    }
}

#[derive(Debug)]
pub enum Frame {
    Text(String),
    Binary(Vec<u8>),
    Close(Option<u16>),
}

/// Provides framing of a websocket 13 connection.
pub struct WebsocketConnection {
    inner: BoxedStream,
}

impl WebsocketConnection {
    /// Creates a new instance of a `WebsocketConnection`
    pub fn new(stream: BoxedStream) -> Self {
        Self { inner: stream }
    }

    /// Sets the timeout of the underlying stream
    pub fn set_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()> {
        self.inner.set_timeout(timeout)
    }

    /// Gets the next websocket frame from the stream. Note: this will error on the default timeout
    /// of the application unless set otherwise using `set_timeout`
    pub fn next_frame(&mut self) -> std::io::Result<Frame> {
        let mut header_bytes = [0u8; 2];
        self.inner.read_exact(&mut header_bytes)?;

        // Todo Handle fragmented frames
        let _fin = header_bytes[0] & 0x80 != 0;
        let opcode = header_bytes[0] & 0x0F;
        let masked = header_bytes[1] & 0x80 != 0;
        let mut payload_len = (header_bytes[1] & 0x7f) as u64;

        if payload_len == 126 {
            let mut extended_payload = [0u8; 2];

            self.inner.read_exact(&mut extended_payload)?;

            payload_len = u16::from_be_bytes(extended_payload) as u64;
        } else if payload_len == 127 {
            let mut extended_payload = [0u8; 8];

            self.inner.read_exact(&mut extended_payload)?;

            payload_len = u64::from_be_bytes(extended_payload) as u64;
        }

        let masking_key = if masked {
            let mut key = [0u8; 4];
            self.inner.read_exact(&mut key)?;
            Some(key)
        } else {
            None
        };

        let mut payload = vec![0; payload_len as usize];
        self.inner.read_exact(&mut payload)?;

        if let Some(key) = masking_key {
            for (i, byte) in payload.iter_mut().enumerate() {
                *byte ^= key[i % 4];
            }
        }

        match opcode {
            0x1 => Ok(Frame::Text(String::from_utf8(payload).map_err(|_| {
                std::io::Error::new(ErrorKind::InvalidData, "Invalid UTF-8 in text frame")
            })?)),
            0x2 => Ok(Frame::Binary(payload)),
            0x8 => {
                let code = if payload.len() >= 2 {
                    Some(u16::from_be_bytes([payload[0], payload[1]]))
                } else {
                    None
                };

                Ok(Frame::Close(code))
            }
            _ => Err(std::io::Error::new(
                ErrorKind::Other,
                "Unsupported frame type",
            )),
        }
    }

    pub fn write_len(&mut self, len: usize) -> std::io::Result<()> {
        if len <= 125 {
            self.inner.write_all(&[len as u8])?
        } else if len <= 65535 {
            self.inner.write_all(&[126])?;
            self.inner.write_all(&len.to_be_bytes())?;
        } else {
            return Err(std::io::Error::new(
                ErrorKind::Other,
                "Payload length exceeds maximum supported length",
            ));
        }

        Ok(())
    }

    pub fn send(&mut self, frame: Frame) -> std::io::Result<()> {
        match frame {
            Frame::Text(data) => {
                self.inner.write_all(&[0x81])?;

                let payload = data.into_bytes();
                self.write_len(payload.len())?;

                self.inner.write_all(&payload)?;
            }
            Frame::Binary(data) => {
                self.inner.write_all(&[0x82])?;

                self.write_len(data.len())?;

                self.inner.write_all(&data)?;
            }
            Frame::Close(code) => {
                self.inner.write_all(&[0x88])?;

                if let Some(code) = code {
                    self.inner.write_all(&[2])?;
                    self.inner.write_all(&code.to_be_bytes())?;
                } else {
                    self.inner.write_all(&[0])?;
                }
            }
        }

        Ok(())
    }
}
