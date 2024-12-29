use super::{
    ecc_operation::ECCOperation,
    message::{EmbassyMessage, MessageKind},
};
use tokio::sync::{broadcast, mpsc};

#[derive(Debug)]
pub enum ConversionError {
    BadString(String),
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadString(s) => write!(f, "Could not convert string {s} to Operation/Status!"),
        }
    }
}

impl std::error::Error for ConversionError {}

#[derive(Debug)]
pub enum EnvoyError {
    BadRequest(reqwest::Error),
    SendError(mpsc::error::SendError<EmbassyMessage>),
    BadConversion(ConversionError),
    FailedMessageParse(serde_json::Error),
    InvalidStringToInt(std::num::ParseIntError),
    InvalidStringToFloat(std::num::ParseFloatError),
    FailedXMLParse(quick_xml::Error),
    FailedXMLUtf8(std::string::FromUtf8Error),
    FailedXMLConvert,
    ServerError(String),
}

impl From<reqwest::Error> for EnvoyError {
    fn from(value: reqwest::Error) -> Self {
        Self::BadRequest(value)
    }
}

impl From<mpsc::error::SendError<EmbassyMessage>> for EnvoyError {
    fn from(value: mpsc::error::SendError<EmbassyMessage>) -> Self {
        Self::SendError(value)
    }
}

impl From<ConversionError> for EnvoyError {
    fn from(value: ConversionError) -> Self {
        Self::BadConversion(value)
    }
}

impl From<serde_json::Error> for EnvoyError {
    fn from(value: serde_json::Error) -> Self {
        Self::FailedMessageParse(value)
    }
}

impl From<std::num::ParseIntError> for EnvoyError {
    fn from(value: std::num::ParseIntError) -> Self {
        Self::InvalidStringToInt(value)
    }
}

impl From<std::num::ParseFloatError> for EnvoyError {
    fn from(value: std::num::ParseFloatError) -> Self {
        Self::InvalidStringToFloat(value)
    }
}

impl From<quick_xml::Error> for EnvoyError {
    fn from(value: quick_xml::Error) -> Self {
        Self::FailedXMLParse(value)
    }
}

impl From<std::string::FromUtf8Error> for EnvoyError {
    fn from(value: std::string::FromUtf8Error) -> Self {
        Self::FailedXMLUtf8(value)
    }
}

impl std::fmt::Display for EnvoyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(e) => {
                write!(f, "Envoy recieved an error while making a request: {e}")
            }
            Self::FailedMessageParse(e) => {
                write!(f, "Envoy failed to parse a message to yaml: {e}")
            }
            Self::BadConversion(e) => write!(f, "Envoy recieved conversion error: {e}"),
            Self::SendError(e) => write!(f, "Envoy failed to send a message: {e}"),
            Self::InvalidStringToInt(e) => {
                write!(f, "Envoy failed to parse string to integer: {e}")
            }
            Self::InvalidStringToFloat(e) => {
                write!(f, "Envoy failed to parse string to float: {e}")
            }
            Self::FailedXMLParse(e) => write!(f, "Envoy failed to parse XML body: {e}"),
            Self::FailedXMLUtf8(e) => write!(f, "Envoy failed to convert XML to String: {e}"),
            Self::FailedXMLConvert => write!(f, "Envoy failed to convert XML data!"),
            Self::ServerError(e) => write!(f, "Server had an internal error: {e}"),
        }
    }
}

impl std::error::Error for EnvoyError {}

#[derive(Debug)]
pub enum EmbassyError {
    FailedMpscSend(mpsc::error::SendError<EmbassyMessage>),
    FailedBroadcastSend(broadcast::error::SendError<EmbassyMessage>),
    InvalidKind(MessageKind, MessageKind),
    FailedParse(serde_json::Error),
    FailedRecieve,
    FailedJoin(tokio::task::JoinError),
    InvalidTransition(ECCOperation),
}

impl From<mpsc::error::SendError<EmbassyMessage>> for EmbassyError {
    fn from(value: mpsc::error::SendError<EmbassyMessage>) -> Self {
        Self::FailedMpscSend(value)
    }
}

impl From<broadcast::error::SendError<EmbassyMessage>> for EmbassyError {
    fn from(value: broadcast::error::SendError<EmbassyMessage>) -> Self {
        Self::FailedBroadcastSend(value)
    }
}

impl From<serde_json::Error> for EmbassyError {
    fn from(value: serde_json::Error) -> Self {
        Self::FailedParse(value)
    }
}

impl From<tokio::task::JoinError> for EmbassyError {
    fn from(value: tokio::task::JoinError) -> Self {
        Self::FailedJoin(value)
    }
}

impl std::fmt::Display for EmbassyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidKind(expected, recieved) => write!(
                f,
                "Embassy expected {expected} message, recieved {recieved} message!"
            ),
            Self::FailedMpscSend(e) => {
                write!(f, "Embassy had an error sending the following message: {e}")
            }
            Self::FailedBroadcastSend(e) => {
                write!(f, "Embassy had an error sending the following message: {e}")
            }
            Self::FailedParse(e) => write!(f, "Embassy had an error parsing a message: {e}"),
            Self::FailedRecieve => {
                write!(f, "Embassy communication lines were disconnected!")
            }
            Self::FailedJoin(e) => write!(f, "Embassy failed to join a task: {e}"),
            Self::InvalidTransition(op) => write!(f, "Attempted invalid transition: {op}"),
        }
    }
}

impl std::error::Error for EmbassyError {}
