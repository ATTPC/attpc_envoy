use super::{
    ecc_operation::ECCOperation,
    message::{EmbassyMessage, MessageKind},
};
use tokio::sync::mpsc::error::SendError;

#[derive(Debug)]
pub enum ECCOperationError {
    BadString(String),
}

impl std::fmt::Display for ECCOperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadString(s) => write!(f, "Could not convert string {s} to ECCOperation!"),
        }
    }
}

impl std::error::Error for ECCOperationError {}

#[derive(Debug)]
pub enum ECCStatusError {
    BadString(String),
}

impl std::fmt::Display for ECCStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadString(s) => write!(f, "Could not convert string {s} to ECCStatus!"),
        }
    }
}

impl std::error::Error for ECCStatusError {}

#[derive(Debug)]
pub enum EnvoyError {
    BadRequest(reqwest::Error),
    SendError(SendError<EmbassyMessage>),
    InvalidStatus(ECCStatusError),
    BadOperation(ECCOperationError),
    FailedMessageParse(serde_yaml::Error),
    InvalidStringToInt(std::num::ParseIntError),
    InvalidStringToFloat(std::num::ParseFloatError),
    FailedXMLParse(quick_xml::Error),
    FailedXMLUtf8(std::string::FromUtf8Error),
    FailedXMLConvert,
}

impl From<reqwest::Error> for EnvoyError {
    fn from(value: reqwest::Error) -> Self {
        Self::BadRequest(value)
    }
}

impl From<SendError<EmbassyMessage>> for EnvoyError {
    fn from(value: SendError<EmbassyMessage>) -> Self {
        Self::SendError(value)
    }
}

impl From<ECCStatusError> for EnvoyError {
    fn from(value: ECCStatusError) -> Self {
        Self::InvalidStatus(value)
    }
}

impl From<ECCOperationError> for EnvoyError {
    fn from(value: ECCOperationError) -> Self {
        Self::BadOperation(value)
    }
}

impl From<serde_yaml::Error> for EnvoyError {
    fn from(value: serde_yaml::Error) -> Self {
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
            Self::BadOperation(e) => write!(f, "Envoy recieved operation error: {e}"),
            Self::InvalidStatus(e) => write!(f, "Envoy recieved status error: {e}"),
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
        }
    }
}

impl std::error::Error for EnvoyError {}

#[derive(Debug)]
pub enum EmbassyError {
    FailedSend(SendError<EmbassyMessage>),
    InvalidKind(MessageKind, MessageKind),
    FailedParse(serde_yaml::Error),
    FailedRecieve,
    FailedJoin(tokio::task::JoinError),
    InvalidTransition(ECCOperation),
}

impl From<SendError<EmbassyMessage>> for EmbassyError {
    fn from(value: SendError<EmbassyMessage>) -> Self {
        Self::FailedSend(value)
    }
}

impl From<serde_yaml::Error> for EmbassyError {
    fn from(value: serde_yaml::Error) -> Self {
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
            Self::FailedSend(e) => {
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
