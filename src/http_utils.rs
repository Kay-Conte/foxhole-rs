use http::{Request, Version, Response};

use std::io::Write;

#[derive(Debug)]
pub enum ParseError {
    InvalidString,
    InvalidSequence,
    InvalidRequestParts,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ParseError::*;

        match self {
            InvalidString => write!(f, "Invalid String"),
            InvalidSequence => write!(f, "Invalid Sequence"),
            InvalidRequestParts => write!(f, "Invalid Request Parts"),
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse_version(s: &str) -> Result<Version, ParseError> {
    Ok(match s {
        "HTTP/0.9" => Version::HTTP_09,
        "HTTP/1.0" => Version::HTTP_10,
        "HTTP/1.1" => Version::HTTP_11,
        "HTTP/2.0" => Version::HTTP_2,
        "HTTP/3.0" => Version::HTTP_3,
        _ => return Err(ParseError::InvalidSequence),
    })
}

pub trait RequestExt {
    fn parse_request(bytes: &[u8]) -> Result<Request<String>, ParseError>;
}

impl RequestExt for Request<String> {
    fn parse_request(bytes: &[u8]) -> Result<Request<String>, ParseError> {
        let raw = std::str::from_utf8(bytes).map_err(|_| ParseError::InvalidString)?;

        let mut parts = raw.split("\r\n\r\n");

        let headers = parts.next().unwrap_or("");
        let body = parts.next().unwrap_or("");

        let mut entries = headers.lines();
        let mut request = entries.next().unwrap_or("").split_whitespace();
        let method = request.next().unwrap_or("");
        let uri = request.next().unwrap_or("");
        let version = parse_version(request.next().unwrap_or(""))?;

        let mut request = Request::builder().method(method).uri(uri).version(version);

        for entry in entries {
            let mut parts = entry.splitn(2, ": ");
            let key = parts.next().unwrap_or("");
            let value = parts.next().unwrap_or("");

            request = request.header(key, value);
        }

        request
            .body(body.to_string())
            .map_err(|_| ParseError::InvalidRequestParts)
    }
}

fn parse_response_line_into_buf<T>(buf: &mut Vec<u8>, request: &Response<T>) -> Result<(), std::io::Error> {
    write!(buf, "HTTP/1.1 \r\n")?;

    for (key, value) in request.headers() {
        buf.write(key.as_str().as_bytes())?;

        write!(buf, ": ")?;

        buf.write(value.as_bytes())?;

        write!(buf, "\r\n")?;
    } 

    write!(buf, "\r\n")?;

    Ok(())
}

pub trait ResponseExt {
    fn to_bytes(self) -> Vec<u8>; 
}

impl ResponseExt for Response<String> {
    fn to_bytes(self) -> Vec<u8> {
        let mut buf = vec![];

        let _ = parse_response_line_into_buf(&mut buf, &self);

        let _ = write!(buf, "\r\n");

        buf.extend_from_slice(self.body().as_bytes());

        buf
    }
}

impl ResponseExt for Response<Vec<u8>> {
    fn to_bytes(self) -> Vec<u8> {
        let mut buf = vec![];

        let _ = parse_response_line_into_buf(&mut buf, &self);

        let _ = write!(buf, "\r\n");

        buf.extend_from_slice(self.body());

        buf
    }
}
