use http::{Request, Version, Response, StatusCode};

use std::io::Write;

use crate::systems::RawResponse;

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

pub trait VersionExt: Sized {
    fn parse_version(s: &str) -> Result<Self, ParseError>;

    fn to_string(&self) -> String;
}

impl VersionExt for Version {
    fn parse_version(s: &str) -> Result<Version, ParseError> {
        Ok(match s {
            "HTTP/0.9" => Version::HTTP_09,
            "HTTP/1.0" => Version::HTTP_10,
            "HTTP/1.1" => Version::HTTP_11,
            "HTTP/2.0" => Version::HTTP_2,
            "HTTP/3.0" => Version::HTTP_3,
            _ => return Err(ParseError::InvalidSequence),
        })
    }

    fn to_string(&self) -> String {
        match self {
            &Version::HTTP_09 => "HTTP/0.9".to_string(),
            &Version::HTTP_10 => "HTTP/1.0".to_string(),
            &Version::HTTP_11 => "HTTP/1.1".to_string(),
            &Version::HTTP_2 => "HTTP/2.0".to_string(),
            &Version::HTTP_3 => "HTTP/3.0".to_string(),
            _ => unreachable!()
        }
    }
}

pub trait RequestFromBytes {
    fn try_from_bytes(bytes: &[u8]) -> Result<Request<String>, ParseError>;
}

impl RequestFromBytes for Request<String> {
    fn try_from_bytes(bytes: &[u8]) -> Result<Request<String>, ParseError> {
        let raw = std::str::from_utf8(bytes).map_err(|_| ParseError::InvalidString)?;

        let mut parts = raw.split("\r\n\r\n");

        let headers = parts.next().unwrap_or("");
        let body = parts.next().unwrap_or("");

        let mut entries = headers.lines();
        let mut request = entries.next().unwrap_or("").split_whitespace();
        let method = request.next().unwrap_or("");
        let uri = request.next().unwrap_or("");
        let version = Version::parse_version(request.next().unwrap_or(""))?;

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
    write!(buf, "{} {} \r\n", request.version().to_string(), request.status().to_string())?;

    for (key, value) in request.headers() {
        buf.write(key.as_str().as_bytes())?;

        write!(buf, ": ")?;

        buf.write(value.as_bytes())?;

        write!(buf, "\r\n")?;
    } 

    write!(buf, "\r\n")?;

    Ok(())
}

pub trait ResponseToBytes {
    fn into_bytes(self) -> Vec<u8>; 
}

impl<T> ResponseToBytes for Response<T> where T: IntoRawBytes {
    fn into_bytes(self) -> Vec<u8> {
        let mut buf = vec![];

        let _ = parse_response_line_into_buf(&mut buf, &self);

        let _ = write!(buf, "\r\n");

        buf.extend_from_slice(self.map(|f| f.into_raw_bytes()).body());

        buf
    }
}

pub trait IntoRawBytes {
    fn into_raw_bytes(self) -> Vec<u8>;
}

impl IntoRawBytes for () {
    fn into_raw_bytes(self) -> Vec<u8> {
        vec![]
    }
}

impl IntoRawBytes for Vec<u8> {
    fn into_raw_bytes(self) -> Vec<u8> {
        self
    }
}

impl IntoRawBytes for String {
    fn into_raw_bytes(self) -> Vec<u8> {
        self.into_bytes()
    }
}

pub trait ResponseExt: Sized {
    fn base(code: StatusCode) -> Response<()>;

    fn empty(code: impl Into<StatusCode>) -> Response<()>;

    fn into_raw_response(self) -> RawResponse;
}

impl<T> ResponseExt for Response<T> where T: IntoRawBytes {
    fn base(code: StatusCode) -> Response<()> {
        Response::builder().status(code).body(()).unwrap()
    }

    fn empty(code: impl Into<StatusCode>) -> Response<()> {
        Response::builder().status(code.into()).body(()).unwrap()
    }

    fn into_raw_response(self) -> RawResponse {
        self.map(|f| f.into_raw_bytes())
    }
}
