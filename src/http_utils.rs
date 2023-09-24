//! This module provides http utility traits and functions for parsing and handling Requests and
//! Responses

use http::{Request, Response, StatusCode, Version};

use std::io::{Write, BufReader, BufRead};

use crate::systems::RawResponse;

#[derive(Debug)]
/// Errors while parsing requests. 
pub enum ParseError {
    MalformedRequest,

    InvalidMethod,
    InvalidProtocolVer,
    InvalidRequestParts,

    /// `Incomplete` should be returned in any case that reading more bytes *may* make the request
    /// valid.
    Incomplete,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidProtocolVer => write!(f, "Invalid Protocol"),
            ParseError::MalformedRequest => write!(f, "Malformed Request"),
            ParseError::Incomplete => write!(f, "Not Enough Bytes"),
            ParseError::InvalidMethod => write!(f, "Invalid Method"),
            ParseError::InvalidRequestParts => write!(f, "Invalid Request Parts"),
        }
    }
}

impl std::error::Error for ParseError {}

pub trait VersionExt: Sized {
    /// # Errors
    ///
    /// Returns `Err` if the `&str` isn't a valid version of the HTTP protocol
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
            _ => return Err(ParseError::InvalidProtocolVer),
        })
    }

    fn to_string(&self) -> String {
        match *self {
            Version::HTTP_09 => "HTTP/0.9".to_string(),
            Version::HTTP_10 => "HTTP/1.0".to_string(),
            Version::HTTP_11 => "HTTP/1.1".to_string(),
            Version::HTTP_2  => "HTTP/2.0".to_string(),
            Version::HTTP_3  => "HTTP/3.0".to_string(),
            _ => unreachable!(),
        }
    }
}

fn validate_method(method: &str) -> bool {
    matches!(method, "GET" | "POST" | "PUT" | "DELETE" | "HEAD" | "OPTIONS" | "CONNECT" | "TRACE" | "PATH")
} 

pub trait RequestFromBytes<'a> {
    /// # Errors
    ///
    /// Returns `Err` if the bytes are not a valid HTTP request
    fn try_headers_from_bytes(bytes: &[u8]) -> Result<Request<()>, ParseError>;
}

/// the entirety of the header must be valid utf8
impl<'a> RequestFromBytes<'a> for Request<&'a [u8]> {
    fn try_headers_from_bytes(bytes: &[u8]) -> Result<Request<()>, ParseError> {
        let mut lines = bytes.lines();

        let Some(Ok(line)) = lines.next() else {
           return Err(ParseError::Incomplete);
        };

        let mut parts = line.split(' ');

        let Some(method) = parts.next() else {
            return Err(ParseError::Incomplete);
        };
        
        if !validate_method(method) {
            return Err(ParseError::InvalidMethod);
        }

        let Some(uri) = parts.next() else {
            return Err(ParseError::Incomplete);            
        };

        let Some(version) = parts.next() else {
            return Err(ParseError::Incomplete);
        };
        
        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .version(Version::parse_version(version)?);

        while let Some(line) = lines.next().transpose().map_err(|_| ParseError::Incomplete)? {
            if line.is_empty() { break; }

            let h = line.split_once(": ")
                .ok_or(ParseError::Incomplete)?;

            if h.1.is_empty() {
                return Err(ParseError::Incomplete);
            }

            req = req.header(
                h.0,
                h.1,
            );
        }

        req.body(()).map_err(|_| ParseError::MalformedRequest)
    }
}


fn parse_response_line_into_buf<T>(
    buf: &mut Vec<u8>,
    request: &Response<T>,
) -> Result<(), std::io::Error> {
    write!(
        buf,
        "{} {} \r\n",
        request.version().to_string(),
        request.status().to_string()
    )?;

    for (key, value) in request.headers() {
        let _ = buf.write(key.as_str().as_bytes())?;

        write!(buf, ": ")?;

        let _ = buf.write(value.as_bytes())?;

        write!(buf, "\r\n")?;
    }

    write!(buf, "\r\n")?;

    Ok(())
}

pub trait ResponseToBytes {
    fn into_bytes(self) -> Vec<u8>;
}

impl<T> ResponseToBytes for Response<T>
where
    T: IntoRawBytes,
{
    fn into_bytes(self) -> Vec<u8> {
        let mut buf = vec![];

        // FIXME idk about not checking result
        let _ = parse_response_line_into_buf(&mut buf, &self);

        let _ = write!(buf, "\r\n");

        buf.extend_from_slice(self.map(IntoRawBytes::into_raw_bytes).body());

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

impl<T> ResponseExt for Response<T>
where
    T: IntoRawBytes,
{
    fn base(code: StatusCode) -> Response<()> {
        Response::builder().status(code).body(()).unwrap()
    }

    fn empty(code: impl Into<StatusCode>) -> Response<()> {
        Response::builder().status(code.into()).body(()).unwrap()
    }

    fn into_raw_response(self) -> RawResponse {
        self.map(IntoRawBytes::into_raw_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn req_headers_parsing_1() {
        let bytes = br#"POST /api/send-data HTTP/1.1\r\nHost: example.com\r\nUser-Agent: My-HTTP-Client/1.0\r\nAccept: application/json\r\nContent-Type: application/json\r\nContent-Length: 87\r\nAuthorization: 82u27ydcfkjegh8jndnkzJJFFJRGHN\r\n\r\n{"user_id":12345, "event_type":"page_view","timestamp":"2023-09-23T15:30:00Z","data":{"page_url":"https://example.com/some-page","user_agent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36","referrer":"https://google.com/search?q=example","browser_language":"en-US","device_type":"desktop"}}"#

        let req = Request::try_headers_from_bytes(bytes);`
        assert!(req.is_ok());

        let req = req.unwrap();
        assert_eq!(req.method, Method::POST);
        assert_eq!(req.path, "/api/send-data");
        assert_eq!(req.version, Version::HTTP_1_1);
        assert_eq!(req.headers.len(), 6);
        assert_eq!(req.headers.get("Host"), Some("example.com"));
        assert_eq!(req.headers.get("User-Agent"), Some("My-HTTP-Client/1.0"));
        assert_eq!(req.headers.get("Accept"), Some("application/json"));
        assert_eq!(req.headers.get("Content-Type"), Some("application/json"));
        assert_eq!(req.headers.get("Content-Length"), Some("87"));
    }

    #[test]
    fn req_headers_parsing_2() {
        let mut bytes = [0; 4096];
        rand::thread_rng().fill_bytes(&mut bytes);

        assert!(Request::try_headers_from_bytes(&bytes[..]).is_err());
    }

    // tests invalid protocol version
    #[test]
    fn resp_headers_parsing_3() {
        let bytes = b"HTTP/1.32 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: 123\r\n\r\n";

        let resp = Response::try_headers_from_bytes(bytes);
        assert_eq!(resp, Err(ParseError::InvalidProtocolVer)));
    }

    // tests invalid header
    #[test]
    fn resp_headers_parsing_4() {
        let bytes = b"HTTP/1.1 200 22 3jklajs OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: 123\r\n\r\n";

        let resp = Response::try_headers_from_bytes(bytes);
        assert_eq!(resp, Err(ParseError::MalformedRequest));
    }

    // missing first line
    #[test]
    fn resp_headers_parsing_5() {
        let bytes = b"Content-Type: text/html; charset=utf-8\r\nContent-Length: 123\r\n\r\n";

        let resp = Response::try_headers_from_bytes(bytes);
        assert_eq!(resp, Err(ParseError::Incomplete));
    }
}
