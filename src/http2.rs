use std::collections::HashMap;
use hpack::Decoder;

use crate::request::{Method, Request};
use crate::response::Response;

pub const HTTP2_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Data,
    Headers,
    Settings,
    Ping,
    GoAway,
    WindowUpdate,
    Unknown(u8),
}

impl From<u8> for FrameType {
    fn from(b: u8) -> Self {
        match b {
            0x0 => FrameType::Data,
            0x1 => FrameType::Headers,
            0x4 => FrameType::Settings,
            0x6 => FrameType::Ping,
            0x7 => FrameType::GoAway,
            0x8 => FrameType::WindowUpdate,
            other => FrameType::Unknown(other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FrameHeader {
    pub length: u32,
    pub frame_type: FrameType,
    pub flags: u8,
    pub stream_id: u32,
}

impl FrameHeader {
    pub fn parse(buf: &[u8]) -> Option<Self> {
        if buf.len() < 9 {
            return None;
        }

        let length = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32);
        let frame_type = FrameType::from(buf[3]);
        let flags = buf[4];
        let stream_id = (((buf[5] as u32) & 0x7F) << 24)
            | ((buf[6] as u32) << 16)
            | ((buf[7] as u32) << 8)
            | (buf[8] as u32);

        Some(Self {
            length,
            frame_type,
            flags,
            stream_id,
        })
    }
}

pub struct Http2Connection {
    pub preface_received: bool,
    hpack_decoder: Decoder<'static>,
    active_streams: HashMap<u32, RequestBuilder>,
    pub connection_window_size: u32,
}

struct RequestBuilder {
    headers: HashMap<String, String>,
    method: Option<Method>,
    path: Option<String>,
    body: Vec<u8>,
    end_stream: bool,
    window_size: u32,
}

impl Default for RequestBuilder {
    fn default() -> Self {
        Self {
            headers: HashMap::new(),
            method: None,
            path: None,
            body: Vec::new(),
            end_stream: false,
            window_size: 65535,
        }
    }
}

impl Http2Connection {
    pub fn new() -> Self {
        Self {
            preface_received: false,
            hpack_decoder: Decoder::new(),
            active_streams: HashMap::new(),
            connection_window_size: 65535,
        }
    }

    /// Check if client payload starts with HTTP/2 connection preface
    pub fn is_http2_preface(buf: &[u8]) -> bool {
        buf.starts_with(HTTP2_PREFACE)
    }

    /// Process HTTP/2 input bytes from `buf`.
    /// Returns `Ok(Some((consumed_bytes, requests)))` or `Ok(None)` if more data is needed.
    pub fn process_input(
        &mut self,
        buf: &[u8],
    ) -> Result<Option<(usize, Vec<(u32, Request)>)>, String> {
        let mut offset = 0;

        if !self.preface_received {
            if buf.len() < HTTP2_PREFACE.len() {
                return Ok(None);
            }
            if !buf.starts_with(HTTP2_PREFACE) {
                return Err("invalid http/2 connection preface".to_string());
            }
            offset += HTTP2_PREFACE.len();
            self.preface_received = true;
        }

        let mut completed_requests = Vec::new();

        while offset + 9 <= buf.len() {
            let header = FrameHeader::parse(&buf[offset..]).unwrap();
            let total_frame_len = 9 + header.length as usize;

            if offset + total_frame_len > buf.len() {
                break;
            }

            let payload = &buf[offset + 9..offset + total_frame_len];
            offset += total_frame_len;

            let is_end_stream = (header.flags & 0x1) != 0;

            match header.frame_type {
                FrameType::Settings => {}
                FrameType::Headers => {
                    let mut stream_builder = self
                        .active_streams
                        .remove(&header.stream_id)
                        .unwrap_or_default();

                    if let Ok(decoded_headers) = self.hpack_decoder.decode(payload) {
                        for (name, val) in decoded_headers {
                            let name_str = String::from_utf8_lossy(&name).to_string();
                            let val_str = String::from_utf8_lossy(&val).to_string();

                            if name_str == ":method" {
                                stream_builder.method = val_str.parse().ok();
                            } else if name_str == ":path" {
                                stream_builder.path = Some(val_str);
                            } else if !name_str.starts_with(':') {
                                stream_builder.headers.insert(name_str, val_str);
                            }
                        }
                    }

                    stream_builder.end_stream = is_end_stream;

                    if is_end_stream {
                        if let (Some(method), Some(path)) =
                            (stream_builder.method, stream_builder.path)
                        {
                            let req = Request {
                                method,
                                path,
                                version: "HTTP/2".to_string(),
                                headers: stream_builder.headers,
                                body: if stream_builder.body.is_empty() {
                                    None
                                } else {
                                    Some(stream_builder.body)
                                },
                            };
                            completed_requests.push((header.stream_id, req));
                        }
                    } else {
                        self.active_streams.insert(header.stream_id, stream_builder);
                    }
                }
                FrameType::Data => {
                    if let Some(mut stream_builder) = self.active_streams.remove(&header.stream_id) {
                        stream_builder.body.extend_from_slice(payload);
                        if is_end_stream {
                            if let (Some(method), Some(path)) =
                                (stream_builder.method, stream_builder.path)
                            {
                                let req = Request {
                                    method,
                                    path,
                                    version: "HTTP/2".to_string(),
                                    headers: stream_builder.headers,
                                    body: if stream_builder.body.is_empty() {
                                        None
                                    } else {
                                        Some(stream_builder.body)
                                    },
                                };
                                completed_requests.push((header.stream_id, req));
                            }
                        } else {
                            self.active_streams.insert(header.stream_id, stream_builder);
                        }
                    }
                }
                FrameType::Ping => {}
                FrameType::WindowUpdate => {
                    if payload.len() >= 4 {
                        let increment = (((payload[0] as u32) & 0x7F) << 24)
                            | ((payload[1] as u32) << 16)
                            | ((payload[2] as u32) << 8)
                            | (payload[3] as u32);
                        if header.stream_id == 0 {
                            self.connection_window_size =
                                self.connection_window_size.saturating_add(increment);
                        } else if let Some(stream_builder) =
                            self.active_streams.get_mut(&header.stream_id)
                        {
                            stream_builder.window_size =
                                stream_builder.window_size.saturating_add(increment);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(Some((offset, completed_requests)))
    }

    /// Generate initial HTTP/2 SETTINGS frame response (9 bytes)
    pub fn build_settings_ack() -> Vec<u8> {
        let mut buf = Vec::with_capacity(9);
        buf.extend_from_slice(&[0, 0, 0, 0x4, 0x1, 0, 0, 0, 0]);
        buf
    }

    /// Encode an HTTP response as HTTP/2 HEADERS + DATA frames for a given stream_id
    pub fn encode_response(stream_id: u32, res: &Response) -> Vec<u8> {
        let mut output = Vec::new();

        let mut header_block = Vec::new();

        let status_str = res.status_code.to_string();
        encode_literal_header(&mut header_block, ":status", &status_str);

        if let Some(ct) = res.content_type {
            encode_literal_header(&mut header_block, "content-type", ct);
        }

        if let Some(ref encoding) = res.content_encoding {
            encode_literal_header(&mut header_block, "content-encoding", encoding);
        }

        if let Some(ref body) = res.body {
            encode_literal_header(&mut header_block, "content-length", &body.len().to_string());
        }

        let has_body = res.body.as_ref().map_or(false, |b| !b.is_empty());
        let headers_flags = if has_body { 0x4 } else { 0x5 };

        write_frame_header(&mut output, header_block.len() as u32, 0x1, headers_flags, stream_id);
        output.extend_from_slice(&header_block);

        if let Some(ref body) = res.body {
            if !body.is_empty() {
                write_frame_header(&mut output, body.len() as u32, 0x0, 0x1, stream_id);
                output.extend_from_slice(body);
            }
        }

        output
    }
}

fn write_frame_header(out: &mut Vec<u8>, length: u32, frame_type: u8, flags: u8, stream_id: u32) {
    out.push(((length >> 16) & 0xFF) as u8);
    out.push(((length >> 8) & 0xFF) as u8);
    out.push((length & 0xFF) as u8);
    out.push(frame_type);
    out.push(flags);
    out.push(((stream_id >> 24) & 0x7F) as u8);
    out.push(((stream_id >> 16) & 0xFF) as u8);
    out.push(((stream_id >> 8) & 0xFF) as u8);
    out.push((stream_id & 0xFF) as u8);
}

fn encode_literal_header(out: &mut Vec<u8>, name: &str, value: &str) {
    out.push(0x00);
    out.push(name.len() as u8);
    out.extend_from_slice(name.as_bytes());
    out.push(value.len() as u8);
    out.extend_from_slice(value.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http2_preface_detection() {
        assert!(Http2Connection::is_http2_preface(HTTP2_PREFACE));
        assert!(!Http2Connection::is_http2_preface(b"GET / HTTP/1.1\r\n\r\n"));
    }

    #[test]
    fn test_frame_header_parse() {
        let mut raw_frame = Vec::new();
        write_frame_header(&mut raw_frame, 15, 0x1, 0x5, 3);
        let header = FrameHeader::parse(&raw_frame).unwrap();
        assert_eq!(header.length, 15);
        assert_eq!(header.frame_type, FrameType::Headers);
        assert_eq!(header.flags, 0x5);
        assert_eq!(header.stream_id, 3);
    }
}
