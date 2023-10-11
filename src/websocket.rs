use std::{net::TcpStream, io::Read};

enum Version {

}

pub enum Frame {
    Text(Vec<u8>),
    Binary(Vec<u8>),
    Close,
    Ping,
    Pong,
}

pub struct Websocket(TcpStream);

impl Websocket {
    pub fn new(stream: TcpStream) -> Self {
        Websocket(stream)
    } 
}

impl Iterator for Websocket {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        let mut header = [0u8; 2];

        self.0.read_exact(&mut header).ok()?;

        let fin = header[0] & 0b1000_0000 != 0;
        let opcode = header[0] & 0b0000_1111;
        let mut len = (header[1] & 0b0111_1111) as usize;

        if !fin {
            todo!("Fragmented frames not yet supported");
        }

        if len == 126 { 
            let mut extension = [0u8; 2];

            self.0.read_exact(&mut extension).ok()?;

            len = u16::from_be_bytes(extension) as usize;
        } else if len == 127 {
            let mut extension = [0u8; 8];

            self.0.read_exact(&mut extension).ok()?;

            len = u64::from_be_bytes(extension) as usize;
        }

        let mut buf = vec![0u8; len];

        self.0.read_exact(&mut buf).ok()?;

        let frame = match opcode {
            0x1 => Frame::Text(buf),
            0x2 => Frame::Binary(buf),
            0x8 => Frame::Close,
            0x9 => Frame::Ping,
            0xA => Frame::Pong,
            _ => return None,
        };

        Some(frame)
    }
}
