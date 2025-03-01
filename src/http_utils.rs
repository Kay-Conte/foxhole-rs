//! This module provides http utility traits and functions for parsing and handling Requests and
//! Responses

use http::{Request, Response, Version};

use std::io::Write;

use crate::action::RawResponse;

/// Errors while parsing requests.
#[derive(Debug)]
pub enum ParseError {
    Unfinished,

    MalformedRequest,

    InvalidEncoding,
    InvalidProtocolVer,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Unfinished => write!(f, "unfinished"),
            ParseError::MalformedRequest => write!(f, "Malformed Request"),
            ParseError::InvalidEncoding => write!(f, "Invalid utf-8"),

            ParseError::InvalidProtocolVer => write!(f, "Invalid Protocol"),
        }
    }
}

impl std::error::Error for ParseError {}

/// `Version` Extension trait
pub trait VersionExt: Sized {
    /// Parse `Version` from a `&str`. Returns `Err` if the `&str` isn't a valid version of the HTTP protocol
    fn parse_version(s: &str) -> Result<Self, ParseError>;

    /// Convert a `Version` to a `String`
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

pub fn take_request(input: &[u8]) -> Result<(Request<()>, usize), ParseError>
where
{
    let mut body_idx = None;
    for (i, line) in input.windows(4).enumerate() {
        if line == b"\r\n\r\n" {
            body_idx = Some(i + 4);
            break;
        }
    }

    let Some(body_idx) = body_idx else {
        return Err(ParseError::Unfinished);
    };

    let Ok(req) = std::str::from_utf8(&input[..body_idx]) else {
        return Err(ParseError::InvalidEncoding);
    };

    let mut lines = req.split("\r\n");

    let req_line = lines.next().ok_or(ParseError::MalformedRequest)?;

    let parts: Vec<&str> = req_line.split(" ").collect();

    if parts.len() < 3 {
        return Err(ParseError::MalformedRequest);
    }

    let method = parts[0];
    let uri = parts[1];
    let version = parts[2];

    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .version(Version::parse_version(version)?);

    for line in lines {
        if line.is_empty() {
            break;
        }

        let header_parts: Vec<&str> = line.splitn(2, ':').collect();

        if header_parts.len() != 2 {
            return Err(ParseError::MalformedRequest);
        }

        builder = builder.header(header_parts[0], header_parts[1])
    }

    Ok((
        builder.body(()).map_err(|_| ParseError::MalformedRequest)?,
        body_idx,
    ))
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
mod test {
    use super::take_request;

    #[test]
    fn test_joined_request() {
        let data = String::from(
            "POST / HTTP/1.0\r\nContent-Length: 0\r\n\r\nPOST / HTTP/1.0\r\nContent-Length: 0\r\n\r\nPOST / HTTP/1.0\r\nContent-Length: 0\r\n\r\n",
        );

        let first = take_request(data.as_bytes()).unwrap();
        assert_eq!(first.1, 38);

        let second = take_request(&data.as_bytes()[first.1..]).unwrap();
        assert_eq!(second.1, 38);

        let third = take_request(&data.as_bytes()[first.1 + second.1..]).unwrap();
        assert_eq!(third.1, 38);
    }

    #[test]
    fn test_request() {
        let data = String::from("POST / HTTP/1.0\r\nContent-Length: 1\r\n\r\n0");

        assert!(take_request(data.as_bytes()).is_ok());
    }
}
