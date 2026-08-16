use std::collections::HashMap;
use std::net::SocketAddr;

use crate::request::{Method, Request};
use crate::response::Response;

/// Header flags for QUIC Packets (RFC 9000)
pub const QUIC_HEADER_FORM_MASK: u8 = 0x80;
pub const QUIC_FIXED_BIT: u8 = 0x40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuicPacketType {
    Initial,
    Handshake,
    Short,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct QuicPacketHeader {
    pub packet_type: QuicPacketType,
    pub dcid: Vec<u8>,
    pub scid: Vec<u8>,
    pub payload_offset: usize,
}

impl QuicPacketHeader {
    pub fn parse(buf: &[u8]) -> Option<Self> {
        if buf.is_empty() {
            return None;
        }

        let first_byte = buf[0];
        let is_long_header = (first_byte & QUIC_HEADER_FORM_MASK) != 0;

        if is_long_header {
            if buf.len() < 12 {
                return None;
            }
            let packet_type = match (first_byte >> 4) & 0x03 {
                0x00 => QuicPacketType::Initial,
                0x02 => QuicPacketType::Handshake,
                _ => QuicPacketType::Unknown,
            };

            let dcid_len = buf[5] as usize;
            if buf.len() < 6 + dcid_len + 1 {
                return None;
            }
            let dcid = buf[6..6 + dcid_len].to_vec();

            let scid_len_offset = 6 + dcid_len;
            let scid_len = buf[scid_len_offset] as usize;
            if buf.len() < scid_len_offset + 1 + scid_len {
                return None;
            }
            let scid = buf[scid_len_offset + 1..scid_len_offset + 1 + scid_len].to_vec();
            let payload_offset = scid_len_offset + 1 + scid_len;

            Some(Self {
                packet_type,
                dcid,
                scid,
                payload_offset,
            })
        } else {
            // Short Header (1-byte header flag + DCID)
            let dcid = buf[1..std::cmp::min(9, buf.len())].to_vec();
            let dcid_len = dcid.len();
            Some(Self {
                packet_type: QuicPacketType::Short,
                dcid,
                scid: Vec::new(),
                payload_offset: 1 + dcid_len,
            })
        }
    }
}

pub struct Http3Connection {
    pub peer_addr: SocketAddr,
    pub active_streams: HashMap<u64, Http3StreamBuilder>,
}

#[derive(Default)]
pub struct Http3StreamBuilder {
    pub headers: HashMap<String, String>,
    pub method: Option<Method>,
    pub path: Option<String>,
    pub body: Vec<u8>,
}

impl Http3Connection {
    pub fn new(peer_addr: SocketAddr) -> Self {
        Self {
            peer_addr,
            active_streams: HashMap::new(),
        }
    }

    /// Process incoming HTTP/3 datagram over UDP.
    /// Returns `Ok(Some(requests))` if valid HTTP/3 request frames are extracted.
    pub fn process_datagram(
        &mut self,
        datagram: &[u8],
    ) -> Result<Vec<(u64, Request)>, String> {
        let mut requests = Vec::new();

        let header = match QuicPacketHeader::parse(datagram) {
            Some(h) => h,
            None => return Ok(requests),
        };

        let payload = &datagram[header.payload_offset..];
        if payload.is_empty() {
            return Ok(requests);
        }

        // QPACK & HTTP/3 Stream Frame Parsing
        // HTTP/3 HEADERS frame (Type 0x01) or DATA frame (Type 0x00)
        let stream_id = 0u64; // Default stream ID 0 for single datagram request
        let mut builder = self.active_streams.remove(&stream_id).unwrap_or_default();

        let mut offset = 0;
        while offset < payload.len() {
            let frame_type = payload[offset];
            offset += 1;

            if offset >= payload.len() {
                break;
            }

            let len = payload[offset] as usize;
            offset += 1;

            if offset + len > payload.len() {
                break;
            }

            let frame_payload = &payload[offset..offset + len];
            offset += len;

            match frame_type {
                0x01 => {
                    // QPACK HEADERS frame
                    parse_qpack_headers(frame_payload, &mut builder);
                }
                0x00 => {
                    // DATA frame
                    builder.body.extend_from_slice(frame_payload);
                }
                _ => {}
            }
        }

        if let (Some(method), Some(path)) = (builder.method, builder.path) {
            let req = Request {
                method,
                path,
                version: "HTTP/3".to_string(),
                headers: builder.headers,
                body: if builder.body.is_empty() {
                    None
                } else {
                    Some(builder.body)
                },
            };
            requests.push((stream_id, req));
        }

        Ok(requests)
    }

    /// Encode an HTTP response as an HTTP/3 UDP Datagram payload
    pub fn encode_response(stream_id: u64, res: &Response) -> Vec<u8> {
        let mut datagram = Vec::new();

        // 1. Write Short Header prefix (0x40 flag)
        datagram.push(0x40);
        datagram.push(stream_id as u8);

        // 2. QPACK HEADERS frame (Type 0x01)
        let mut header_block = Vec::new();
        let status_str = res.status_code.to_string();
        encode_qpack_field(&mut header_block, ":status", &status_str);

        if let Some(ct) = res.content_type {
            encode_qpack_field(&mut header_block, "content-type", ct);
        }

        if let Some(ref encoding) = res.content_encoding {
            encode_qpack_field(&mut header_block, "content-encoding", encoding);
        }

        if let Some(ref body) = res.body {
            encode_qpack_field(&mut header_block, "content-length", &body.len().to_string());
        }

        // HEADERS Frame: Type 0x01, Length, Payload
        datagram.push(0x01);
        datagram.push(header_block.len() as u8);
        datagram.extend_from_slice(&header_block);

        // 3. DATA Frame (Type 0x00) if body exists
        if let Some(ref body) = res.body {
            if !body.is_empty() {
                datagram.push(0x00);
                datagram.push(body.len() as u8);
                datagram.extend_from_slice(body);
            }
        }

        datagram
    }
}

fn parse_qpack_headers(payload: &[u8], builder: &mut Http3StreamBuilder) {
    let mut i = 0;
    while i < payload.len() {
        if payload[i] == 0x00 {
            i += 1;
            if i >= payload.len() {
                break;
            }

            let name_len = payload[i] as usize;
            i += 1;
            if i + name_len > payload.len() {
                break;
            }
            let name = String::from_utf8_lossy(&payload[i..i + name_len]).to_string();
            i += name_len;

            if i >= payload.len() {
                break;
            }

            let val_len = payload[i] as usize;
            i += 1;
            if i + val_len > payload.len() {
                break;
            }
            let val = String::from_utf8_lossy(&payload[i..i + val_len]).to_string();
            i += val_len;

            if name == ":method" {
                builder.method = val.parse().ok();
            } else if name == ":path" {
                builder.path = Some(val);
            } else if !name.starts_with(':') {
                builder.headers.insert(name, val);
            }
        } else {
            i += 1;
        }
    }
}

fn encode_qpack_field(out: &mut Vec<u8>, name: &str, value: &str) {
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
    fn test_quic_packet_header_parse() {
        let raw = vec![0x80, 1, 2, 3, 4, 4, 10, 20, 30, 40, 4, 50, 60, 70, 80, 0, 1, 2];
        let header = QuicPacketHeader::parse(&raw).unwrap();
        assert_eq!(header.packet_type, QuicPacketType::Initial);
        assert_eq!(header.dcid, vec![10, 20, 30, 40]);
        assert_eq!(header.scid, vec![50, 60, 70, 80]);
    }
}
