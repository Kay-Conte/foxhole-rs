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

fn split_lines(input: &[u8]) -> Vec<&[u8]> {
    let mut results = Vec::new();
    let mut last = 0;

    if input.len() < 2 {
        return vec![input];
    }

    for i in 0..input.len() - 1 {
        if input[i] == b'\r' && input[i + 1] == b'\n' {
            results.push(&input[last..i]);

            last = i + 2;
        }
    }

    if last < input.len() {
        results.push(&input[last..]);
    }

    results
}

/// Read a request from a source
pub fn take_request(input: &[u8]) -> Result<(Request<()>, usize), ParseError>
where
{
    let mut body_index = 0;

    let mut lines = split_lines(input).into_iter();

    let request_line = lines.next().ok_or(ParseError::Unfinished)?;

    if request_line.is_empty() {
        return Err(ParseError::Unfinished);
    }

    // Adjust for "\r\n"
    body_index += request_line.len() + 2;

    let parts: Vec<&[u8]> = request_line.split(|&b| b == b' ').collect();

    if parts.len() < 3 {
        return Err(ParseError::MalformedRequest);
    }

    let method = std::str::from_utf8(parts[0]).map_err(|_| ParseError::InvalidEncoding)?;
    let uri = std::str::from_utf8(parts[1]).map_err(|_| ParseError::InvalidEncoding)?;
    let version = std::str::from_utf8(parts[2]).map_err(|_| ParseError::InvalidEncoding)?;

    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .version(Version::parse_version(version)?);

    for line in lines {
        if line.is_empty() || line.starts_with(&[b'\r']) {
            break;
        }

        body_index += line.len() + 2;

        let header_parts: Vec<&[u8]> = line.splitn(2, |&b| b == b':').collect();

        if header_parts.len() != 2 {
            return Err(ParseError::MalformedRequest);
        }

        let key = std::str::from_utf8(header_parts[0]).map_err(|_| ParseError::InvalidEncoding)?;
        let value = std::str::from_utf8(header_parts[1])
            .map_err(|_| ParseError::InvalidEncoding)?
            .trim();

        req = req.header(key, value);
    }

    // Adjust for final "\r\n"
    body_index += 2;

    Ok((
        req.body(()).map_err(|_| ParseError::MalformedRequest)?,
        body_index,
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
