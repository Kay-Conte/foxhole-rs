//! This module provides http utility traits and functions for parsing and handling Requests and
//! Responses

use http::{Request, Response, Version};

use std::io::{BufRead, Write};

use crate::action::RawResponse;

/// Errors while parsing requests.
#[derive(Debug, PartialEq)]
pub enum ParseError {
    MalformedRequest,
    ReadError,

    InvalidProtocolVer,
    InvalidRequestParts,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::MalformedRequest => write!(f, "Malformed Request"),
            ParseError::ReadError => write!(f, "Read Error"),

            ParseError::InvalidProtocolVer => write!(f, "Invalid Protocol"),
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
            Version::HTTP_2 => "HTTP/2.0".to_string(),
            Version::HTTP_3 => "HTTP/3.0".to_string(),
            _ => unreachable!(),
        }
    }
}

/// the entirety of the header must be valid utf8
pub fn take_request<R>(reader: &mut R) -> Result<Request<()>, ParseError>
where
    R: BufRead,
{
    let mut lines = reader.lines();

    let line = lines
        .next()
        .ok_or(ParseError::MalformedRequest)?
        .map_err(|_| ParseError::ReadError)?;

    let mut parts = line.split(' ');

    let method = parts.next().ok_or(ParseError::MalformedRequest)?;

    let uri = parts.next().ok_or(ParseError::MalformedRequest)?;

    let version = parts.next().ok_or(ParseError::MalformedRequest)?;

    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .version(Version::parse_version(version)?);

    while let Some(line) = lines
        .next()
        .transpose()
        .map_err(|_| ParseError::ReadError)?
    {
        if line.is_empty() {
            break;
        }

        let h = line.split_once(": ").ok_or(ParseError::MalformedRequest)?;

        if h.1.is_empty() {
            return Err(ParseError::MalformedRequest);
        }

        req = req.header(h.0, h.1);
    }

    req.body(()).map_err(|_| ParseError::MalformedRequest)
}

fn parse_response_line_into_buf<T>(
    buf: &mut Vec<u8>,
    request: &Response<T>,
) -> Result<(), std::io::Error> {
    write!(
        buf,
        "{} {}\r\n",
        request.version().to_string(),
        request.status()
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

impl<T> IntoRawBytes for Response<T>
where
    T: IntoRawBytes,
{
    fn into_raw_bytes(self) -> Vec<u8> {
        let mut buf = vec![];

        // FIXME idk about not checking result
        let _ = parse_response_line_into_buf(&mut buf, &self);

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
    fn into_raw_response(self) -> RawResponse;
}

impl<T> ResponseExt for Response<T>
where
    T: IntoRawBytes,
{
    fn into_raw_response(self) -> RawResponse {
        self.map(IntoRawBytes::into_raw_bytes)
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use super::*;
    use http::{HeaderValue, Method, Version};

    #[test]
    fn sanity_check() {
        let bytes = b"POST /api/send-data HTTP/1.1 \r\nHost: example.com\r\nUser-Agent: My-HTTP-Client/1.0\r\nAccept: application/json\r\nContent-Type: application/json\r\nContent-Length: 87\r\nAuthorization: 82u27ydcfkjegh8jndnkzJJFFJRGHN\r\n\r\n{\"user_id\":12345, \"event_type\":\"page_view\",\"timestamp\":\"2023-09-23T15:30:00Z\",\"data\":{\"page_url\":\"https://example.com/some-page\",\"user_agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36\",\"referrer\":\"https://google.com/search?q=example\",\"browser_language\":\"en-US\",\"device_type\":\"desktop\"}}";
        let mut reader = BufReader::new(&bytes[..]);

        let req = take_request(&mut reader);
        assert!(req.is_ok());

        let req = req.unwrap();
        assert_eq!(req.method(), Method::POST);
        assert_eq!(req.uri(), "/api/send-data");
        assert_eq!(req.version(), Version::HTTP_11);
        assert_eq!(req.headers().len(), 6);
        assert_eq!(
            req.headers().get("Host"),
            Some(&HeaderValue::from_str("example.com").unwrap())
        );
        assert_eq!(
            req.headers().get("User-Agent"),
            Some(&HeaderValue::from_str("My-HTTP-Client/1.0").unwrap())
        );
        assert_eq!(
            req.headers().get("Accept"),
            Some(&HeaderValue::from_str("application/json").unwrap())
        );
        assert_eq!(
            req.headers().get("Content-Type"),
            Some(&HeaderValue::from_str("application/json").unwrap())
        );
        assert_eq!(
            req.headers().get("Content-Length"),
            Some(&HeaderValue::from_str("87").unwrap())
        );
    }

    #[test]
    fn invalid_protocol() {
        let bytes = b"GET / HTTP/1.32 \r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: 123\r\n\r\n";
        let mut reader = BufReader::new(&bytes[..]);

        let resp = take_request(&mut reader);
        assert!(matches!(resp, Err(ParseError::InvalidProtocolVer)));
    }

    #[test]
    fn invalid_header() {
        let bytes = b"GET / jjshjudh HTTP/1.1 22 3jklajs \r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: 123\r\n\r\n";

        let mut reader = BufReader::new(&bytes[..]);

        let resp = take_request(&mut reader);
        assert!(matches!(resp, Err(ParseError::InvalidProtocolVer)));
    }

    #[test]
    fn invalid_method() {
        let bytes = b"BAKLAVA / HTTP/1.1 \r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: 123\r\n\r\n";

        let mut reader = BufReader::new(&bytes[..]);

        let resp = take_request(&mut reader);
        assert!(matches!(resp, Err(ParseError::MalformedRequest)));
    }

    #[test]
    fn not_enough_bytes() {
        let bytes = b"GET / HTTP/1.1 \r\nContent-Type: \r\n\r\n";
        let mut reader = BufReader::new(&bytes[..]);

        let resp = take_request(&mut reader);
        assert!(matches!(resp, Err(ParseError::MalformedRequest)));

        let bytes = b"GET / HTTP/1.1 \r\nContent-Type: \r\nContent-L";
        let mut reader = BufReader::new(&bytes[..]);

        let resp = take_request(&mut reader);
        assert!(matches!(resp, Err(ParseError::MalformedRequest)));
    }

    #[test]
    fn unicode_in_request() {
        let bytes = b"GET / HTTP/1.1 \r\nContent-Type: \xE2\xA1\x91\xE2\xB4\x9D\xE2\x9B\xB6\r\nContent-Length: 123\r\n\r\n";
        let mut reader = BufReader::new(&bytes[..]);

        let resp = take_request(&mut reader);
        assert!(resp.is_ok());
    }

    #[test]
    fn malformed_request() {
        // missing header field
        let bytes = b"GET / HTTP/1.1 \r\nContent-Type: \r\nContent-Length: 123\r\n\r\n";
        let mut reader = BufReader::new(&bytes[..]);

        let resp = take_request(&mut reader);
        assert!(matches!(resp, Err(ParseError::MalformedRequest)));

        // incomplete header key
        let bytes = b"GET / HTTP/1.1\r\nCey: text/html; charset=utf-8\r\nContent-Len\r\n\r\n";
        let mut reader = BufReader::new(&bytes[..]);

        let resp = take_request(&mut reader);
        assert!(matches!(resp, Err(ParseError::MalformedRequest)));
    }

    #[test]
    fn nulls_in_request() {
        let bytes = b"GET / HTTP/1.1\r\nContent-Type: \0\r\nContent-Length: 123\r\n\r\n";
        let mut reader = BufReader::new(&bytes[..]);

        let resp = take_request(&mut reader);
        assert!(matches!(resp, Err(_)));

        let bytes =
            b"GET / HTTP/1.1\r\n\0: text/html; charset=utf-8\r\nContent-Length: 123\r\n\r\n";
        let mut reader = BufReader::new(&bytes[..]);

        let resp = take_request(&mut reader);
        assert!(matches!(resp, Err(_)));

        let bytes = b"\0 \0 \0\r\n\0: text/html; charset=utf-8\r\nContent-Length: 123\r\n\r\n";
        let mut reader = BufReader::new(&bytes[..]);

        let resp = take_request(&mut reader);
        assert!(matches!(resp, Err(_)));
    }
}
