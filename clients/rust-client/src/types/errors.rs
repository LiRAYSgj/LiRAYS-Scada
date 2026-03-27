use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("no active connection")]
    NotConnected,
    #[error("connection closed")]
    ConnectionClosed,
    #[error("unexpected or malformed frame")]
    UnknownFrame,
    #[error("expected a response but got something else")]
    UnexpectedFrame,
    #[error("invalid input: {0}")]
    InvalidInput(&'static str),
    #[error("operation failed: {0}")]
    OperationFailed(String),
    #[error("timeout waiting for response")]
    Timeout,
    #[error("unexpected text message: {0}")]
    UnexpectedText(String),
    #[error("websocket error: {0}")]
    Ws(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("protobuf decode error: {0}")]
    Decode(#[from] prost::DecodeError),
}
