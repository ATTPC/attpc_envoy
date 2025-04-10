use super::{
    ecc_operation::ECCOperation,
    message::{EmbassyMessage, MessageKind},
};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc};

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("Could not convert string {0} to Operation/Status")]
    BadString(String),
}

#[derive(Debug, Error)]
pub enum EnvoyError {
    #[error("Bad request: {0}")]
    BadRequest(#[from] reqwest::Error),
    #[error("Envoy failed to send a message: {0}")]
    SendError(#[from] mpsc::error::SendError<EmbassyMessage>),
    #[error("Message conversion failed: {0}")]
    BadConversion(#[from] ConversionError),
    #[error("Failed to parse message JSON: {0}")]
    FailedMessageParse(#[from] serde_json::Error),
    #[error("Failed to parse message integer: {0}")]
    InvalidStringToInt(#[from] std::num::ParseIntError),
    #[error("Failed to parse message float: {0}")]
    InvalidStringToFloat(#[from] std::num::ParseFloatError),
    #[error("Failed to parse message XML: {0}")]
    FailedXMLParse(#[from] quick_xml::Error),
    #[error("Failed to parse message XML as utf8: {0}")]
    FailedXMLUtf8(#[from] std::string::FromUtf8Error),
    #[error("Failed to parse message XML conversion")]
    FailedXMLConvert,
    #[error("Sentry server had an internal error: {0}")]
    ServerError(String),
}

#[derive(Debug, Error)]
pub enum EmbassyError {
    #[error("Embassy failed to send message: {0}")]
    FailedMpscSend(#[from] mpsc::error::SendError<EmbassyMessage>),
    #[error("Embassy failed to send cancel: {0}")]
    FailedBroadcastSend(#[from] broadcast::error::SendError<EmbassyMessage>),
    #[error("Embassy received invalid message kind: {0} expected: {1}")]
    InvalidKind(MessageKind, MessageKind),
    #[error("Embassy failed to parse message JSON: {0}")]
    FailedParse(#[from] serde_json::Error),
    #[error("Embassy failed to receive messages")]
    FailedRecieve,
    #[error("Embassy failed to join tasks: {0}")]
    FailedJoin(#[from] tokio::task::JoinError),
    #[error("Embassy received an invalid operation: {0}")]
    InvalidTransition(ECCOperation),
}

#[derive(Debug, Error)]
pub enum BackupError {
    #[error("Backup of config files failed due to: {0}")]
    FailedIO(#[from] std::io::Error),
    #[error("Config backup dir {0:?} already exists for run {1}")]
    AlreadyExists(std::path::PathBuf, i32),
}
