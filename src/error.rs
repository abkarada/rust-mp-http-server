use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("malformed request: {0}")]
    MalformedRequest(&'static str),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
