use std::io::ErrorKind;

use crate::{action::IntoAction, connection::BoxedStream, Action, Resolve, ResolveGuard};

pub struct Upgrade;

impl Upgrade {
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
        let Some(header) = ctx.request.headers().get("connection") else {
            return ResolveGuard::None;
        };

        if header == "upgrade" {
            ResolveGuard::Value(Upgrade)
        } else {
            ResolveGuard::None
        }
    }
}

pub struct Websocket(fn(WebsocketConnection));

impl IntoAction for Websocket {
    fn action(self) -> crate::Action {
        Action::Upgrade(Box::new(move |stream| {
            // Create websocket connection type
            // Call function with connection
            (self.0)(WebsocketConnection::new(stream))
        }))
    }
}

#[derive(Debug)]
pub enum Frame {
    Text(String),
    Binary(Vec<u8>),
    Close(Option<u16>),
}

pub struct WebsocketConnection {
    inner: BoxedStream,
}

impl WebsocketConnection {
    pub fn new(stream: BoxedStream) -> Self {
        Self { inner: stream }
    }

    pub fn next_frame(&mut self) -> std::io::Result<Frame> {
        let mut header_bytes = [0u8; 2];
        self.inner.read_exact(&mut header_bytes)?;

        let fin = header_bytes[0] & 0x80 != 0;
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
}
