use std::io::Read;
use flate2::read::GzEncoder;
use flate2::Compression;

/// Supported encoding formats for HTTP Content-Encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Gzip,
}

impl Encoding {
    /// Parse Accept-Encoding header string and return matching Encoding if supported
    pub fn parse_accept_encoding(header: &str) -> Option<Self> {
        for part in header.split(',') {
            if part.trim().eq_ignore_ascii_case("gzip") {
                return Some(Encoding::Gzip);
            }
        }
        None
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Encoding::Gzip => "gzip",
        }
    }
}

/// Compress input bytes using specified encoding
pub fn compress(data: &[u8], encoding: Encoding) -> Vec<u8> {
    match encoding {
        Encoding::Gzip => {
            let mut encoder = GzEncoder::new(data, Compression::default());
            let mut compressed = Vec::new();
            let _ = encoder.read_to_end(&mut compressed);
            compressed
        }
    }
}
