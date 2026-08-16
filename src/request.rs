use std::{collections::HashMap, fmt, str::FromStr};

use crate::error::HttpError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Patch,
    Unknown(String),
}

impl Method {
    pub fn as_str(&self) -> &str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
            Method::Patch => "PATCH",
            Method::Unknown(s) => s.as_str(),
        }
    }
}

impl FromStr for Method {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_uppercase().as_str() {
            "GET" => Method::Get,
            "POST" => Method::Post,
            "PUT" => Method::Put,
            "DELETE" => Method::Delete,
            "HEAD" => Method::Head,
            "OPTIONS" => Method::Options,
            "PATCH" => Method::Patch,
            other => Method::Unknown(other.to_string()),
        })
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub struct Request {
    pub method: Method,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

impl Request {
    /// Zero-copy HTTP header parsing over raw byte slices using `httparse`.
    /// Returns `Ok(Some((Request, bytes_consumed)))` if request is complete,
    /// `Ok(None)` if more data needs to be read from socket, or `Err` if malformed.
    pub fn parse(buf: &[u8]) -> Result<Option<(Self, usize)>, HttpError> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);

        match req.parse(buf) {
            Ok(httparse::Status::Complete(header_len)) => {
                let method_str = req.method.unwrap_or("GET");
                let method = Method::from_str(method_str).unwrap();
                let path = req.path.unwrap_or("/").to_string();
                let version = format!("HTTP/1.{}", req.version.unwrap_or(1));

                let mut header_map = HashMap::new();
                for h in req.headers.iter() {
                    if let Ok(value_str) = std::str::from_utf8(h.value) {
                        header_map.insert(h.name.to_string(), value_str.to_string());
                    }
                }

                // Check content length if body exists
                let content_length = header_map
                    .get("Content-Length")
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(0);

                let total_expected_len = header_len + content_length;
                if buf.len() < total_expected_len {
                    // Need more body data
                    return Ok(None);
                }

                let body_bytes = &buf[header_len..total_expected_len];
                let body = if !body_bytes.is_empty() {
                    Some(body_bytes.to_vec())
                } else {
                    None
                };

                Ok(Some((
                    Request {
                        method,
                        path,
                        version,
                        headers: header_map,
                        body,
                    },
                    total_expected_len,
                )))
            }
            Ok(httparse::Status::Partial) => Ok(None),
            Err(_) => Err(HttpError::MalformedRequest("invalid http request format")),
        }
    }
}

impl FromStr for Request {
    type Err = HttpError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match Self::parse(text.as_bytes())? {
            Some((req, _len)) => Ok(req),
            None => Err(HttpError::MalformedRequest("partial request")),
        }
    }
}
