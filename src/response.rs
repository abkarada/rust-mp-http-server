use std::io::Write;

use crate::compression::{self, Encoding};
use crate::request::Request;

pub struct Response {
    pub version: String,
    pub status_code: u16,
    pub reason: &'static str,
    pub content_encoding: Option<String>,
    pub content_type: Option<&'static str>,
    pub body: Option<Vec<u8>>,
    pub connection: Option<String>,
}

impl Response {
    pub fn ok(req: &Request) -> Self {
        Self {
            version: req.version.clone(),
            status_code: 200,
            reason: "OK",
            content_encoding: None,
            content_type: None,
            body: None,
            connection: None,
        }
    }

    pub fn created(req: &Request) -> Self {
        Self {
            version: req.version.clone(),
            status_code: 201,
            reason: "Created",
            content_encoding: None,
            content_type: None,
            body: None,
            connection: None,
        }
    }

    pub fn no_content(req: &Request) -> Self {
        Self {
            version: req.version.clone(),
            status_code: 204,
            reason: "No Content",
            content_encoding: None,
            content_type: None,
            body: None,
            connection: None,
        }
    }

    pub fn create_error(req: &Request) -> Self {
        Self {
            version: req.version.clone(),
            status_code: 500,
            reason: "Internal Server Error",
            content_encoding: None,
            content_type: None,
            body: None,
            connection: None,
        }
    }

    pub fn not_found() -> Self {
        Self {
            version: "HTTP/1.1".to_string(),
            status_code: 404,
            reason: "Not Found",
            content_encoding: None,
            content_type: None,
            body: None,
            connection: None,
        }
    }

    pub fn method_not_allowed() -> Self {
        Self {
            version: "HTTP/1.1".to_string(),
            status_code: 405,
            reason: "Method Not Allowed",
            content_encoding: None,
            content_type: None,
            body: None,
            connection: None,
        }
    }

    pub fn with_text_body(self, body: String, req: &Request) -> Self {
        self.with_body(body.into_bytes(), "text/plain", req)
    }

    pub fn with_octet_stream(self, body: Vec<u8>, req: &Request) -> Self {
        self.with_body(body, "application/octet-stream", req)
    }

    pub fn with_typed_body(self, body: Vec<u8>, content_type: &'static str, req: &Request) -> Self {
        self.with_body(body, content_type, req)
    }

    /// Internal helper to attach body, automatically compressing if Accept-Encoding header matches.
    fn with_body(mut self, body: Vec<u8>, content_type: &'static str, req: &Request) -> Self {
        self.content_type = Some(content_type);

        let accepted_encoding = req
            .headers
            .get("Accept-Encoding")
            .and_then(|h| Encoding::parse_accept_encoding(h));

        if let Some(encoding) = accepted_encoding {
            self.content_encoding = Some(encoding.as_str().to_string());
            self.body = Some(compression::compress(&body, encoding));
        } else {
            self.body = Some(body);
        }

        self
    }

    /// Evaluates Connection header from Request and sets connection header on Response.
    /// Returns `true` if the connection should be closed after writing.
    pub fn apply_connection_header(&mut self, req: &Request) -> bool {
        if let Some(conn) = req.headers.get("Connection") {
            if conn.eq_ignore_ascii_case("close") {
                self.connection = Some("close".to_string());
                return true;
            }
        }
        false
    }

    pub fn write_to_stream<W: Write>(&self, stream: &mut W) -> std::io::Result<()> {
        write!(stream, "{} {} {}\r\n", self.version, self.status_code, self.reason)?;

        if let Some(ref encoding) = self.content_encoding {
            write!(stream, "Content-Encoding: {}\r\n", encoding)?;
        }

        if let Some(ct) = self.content_type {
            let len = self.body.as_ref().map_or(0, |b| b.len());
            write!(stream, "Content-Type: {}\r\n", ct)?;
            write!(stream, "Content-Length: {}\r\n", len)?;
        }

        if let Some(ref conn) = self.connection {
            write!(stream, "Connection: {}\r\n", conn)?;
        }

        write!(stream, "\r\n")?;

        if let Some(ref body) = self.body {
            stream.write_all(body)?;
        }

        stream.flush()?;
        Ok(())
    }
}

/// Utility function to guess MIME content type based on file extension
pub fn mime_type_for_path(path: &str) -> &'static str {
    if path.ends_with(".html") || path.ends_with(".htm") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".js") {
        "text/javascript"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".txt") {
        "text/plain"
    } else {
        "application/octet-stream"
    }
}
