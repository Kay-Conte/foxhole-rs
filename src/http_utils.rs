//! This module provides http utility traits and functions for parsing and handling Requests and
//! Responses

use http::{Request, Version, Response, StatusCode};

use std::io::{Write, BufReader, BufRead, Read};

use crate::systems::RawResponse;

#[derive(Debug)]
pub enum ParseError {
    InvalidString,

    MalformedRequest,
    ExpectedMethod,
    InvalidMethod,
    InvalidProtocolVer,

    NoContentLength,
    InvalidRequestParts,

    NotEnoughBytes,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidString => write!(f, "Invalid String"),
            ParseError::InvalidProtocolVer => write!(f, "Invalid Protocol"),
            ParseError::MalformedRequest => write!(f, "Malformed Request"),
            ParseError::NotEnoughBytes => write!(f, "Not Enough Bytes"),
            ParseError::ExpectedMethod => write!(f, "Expected Method"),
            ParseError::InvalidMethod => write!(f, "Invalid Method"),
            ParseError::NoContentLength => write!(f, "No Content Length"),
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
            Version::HTTP_09 => String::from("HTTP/0.9"),
            Version::HTTP_10 => String::from("HTTP/1.0"),
            Version::HTTP_11 => String::from("HTTP/1.1"),
            Version::HTTP_2  => String::from("HTTP/2.0"),
            Version::HTTP_3  => String::from("HTTP/3.0"),
            _ => unreachable!(),
        }
    }
}

pub trait RequestFromBytes<'a> {
    /// # Errors
    ///
    /// Returns `Err` if the bytes are not a valid HTTP request
    fn try_from_bytes(bytes: &[u8]) -> Result<Request<Vec<u8>>, ParseError>;
}

impl<'a> RequestFromBytes<'a> for Request<&'a [u8]> {
    fn try_from_bytes(bytes: &[u8]) -> Result<Request<Vec<u8>>, ParseError> {
        let mut reader = BufReader::new(bytes);


        //
        // request line
        let (method, uri, version): (String, String, Version);
        let mut buf = String::new();

        reader.read_line(&mut buf).map_err(|_| ParseError::NotEnoughBytes)?;

        let h = buf.split_whitespace().collect::<Vec<&str>>();

        if h.len() < 3 { 
            return Err(ParseError::MalformedRequest); 
        }

        let meth = *h.first().ok_or(ParseError::ExpectedMethod)?;
        match meth {
            "GET" | "POST" | "PUT" | "DELETE" | "HEAD" | "OPTIONS" | "CONNECT" | "TRACE" | "PATH" 
                => method = meth.to_string(),
            _ => return Err(ParseError::InvalidMethod),
        }

        version = Version::parse_version(h[2])?;
        uri = h.get(1).ok_or(ParseError::MalformedRequest)?.to_string();


        //
        // headers
        let mut headers: Vec<(String, String)> = Vec::new();
        let mut buf = String::new();
        loop { 
            reader.read_line(&mut buf).map_err(|_| ParseError::NotEnoughBytes)?;

            if buf == "\r\n" { break; }

            let Some(h) = buf.split_once(": ") else {
                return Err(ParseError::NotEnoughBytes);
            };

            if h.1.trim().is_empty() {
                return Err(ParseError::NotEnoughBytes);
            }

            headers.push((h.0.to_string(), h.1.to_string()));
        }

        let mut request = Request::builder()
            .method(method.as_bytes())
            .uri(uri)
            .version(version);

        let mut content_length = None;
        for (key, value) in headers {
            if key == "Content-Length" {
                content_length = Some(value.parse::<usize>().map_err(|_| ParseError::NotEnoughBytes)?);
            }
            request = request.header(key, value);
        }

        //
        // body
        if content_length.is_none() {
            return Err(ParseError::NoContentLength);
        }

        let mut body = vec![0; content_length.unwrap()];
        reader.read_exact(&mut body).map_err(|_| ParseError::NotEnoughBytes)?;


        request.body(body).map_err(|_| ParseError::InvalidRequestParts)
    }
}

fn parse_response_line_into_buf<T>(buf: &mut Vec<u8>, request: &Response<T>) -> Result<(), std::io::Error> {
    write!(buf, "{} {} \r\n", request.version().to_string(), request.status())?;

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

impl<T> ResponseToBytes for Response<T> where T: IntoRawBytes {
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

impl<T> ResponseExt for Response<T> where T: IntoRawBytes {
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
