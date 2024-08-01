use super::message::{EmbassyMessage, MessageKind};
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
pub enum FribOperationError {
    BadString(String),
}

impl std::fmt::Display for FribOperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadString(s) => write!(f, "Could not convert string {s} to FribOperation!"),
        }
    }
}

impl std::error::Error for FribOperationError {}

#[derive(Debug)]
pub enum FribStatusError {
    BadString(String),
}

impl std::fmt::Display for FribStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadString(s) => write!(f, "Could not convert string {s} to FribStatus!"),
        }
    }
}

impl std::error::Error for FribStatusError {}

#[derive(Debug)]
pub enum EnvoyError {
    RequestError(reqwest::Error),
    SendError(SendError<EmbassyMessage>),
    ECCStatusError(ECCStatusError),
    FribStatusError(FribStatusError),
    ECCOperationError(ECCOperationError),
    FribOperationError(FribOperationError),
    MessageParseError(serde_yaml::Error),
    StringToIntError(std::num::ParseIntError),
    StringToFloatError(std::num::ParseFloatError),
    XMLError(quick_xml::Error),
    XMLUtf8Error(std::string::FromUtf8Error),
    XMLConversionError,
    TCPConnectionError,
}

impl From<reqwest::Error> for EnvoyError {
    fn from(value: reqwest::Error) -> Self {
        Self::RequestError(value)
    }
}

impl From<SendError<EmbassyMessage>> for EnvoyError {
    fn from(value: SendError<EmbassyMessage>) -> Self {
        Self::SendError(value)
    }
}

impl From<ECCStatusError> for EnvoyError {
    fn from(value: ECCStatusError) -> Self {
        Self::ECCStatusError(value)
    }
}

impl From<ECCOperationError> for EnvoyError {
    fn from(value: ECCOperationError) -> Self {
        Self::ECCOperationError(value)
    }
}

impl From<FribOperationError> for EnvoyError {
    fn from(value: FribOperationError) -> Self {
        Self::FribOperationError(value)
    }
}

impl From<FribStatusError> for EnvoyError {
    fn from(value: FribStatusError) -> Self {
        Self::FribStatusError(value)
    }
}

impl From<serde_yaml::Error> for EnvoyError {
    fn from(value: serde_yaml::Error) -> Self {
        Self::MessageParseError(value)
    }
}

impl From<std::num::ParseIntError> for EnvoyError {
    fn from(value: std::num::ParseIntError) -> Self {
        Self::StringToIntError(value)
    }
}

impl From<std::num::ParseFloatError> for EnvoyError {
    fn from(value: std::num::ParseFloatError) -> Self {
        Self::StringToFloatError(value)
    }
}

impl From<quick_xml::Error> for EnvoyError {
    fn from(value: quick_xml::Error) -> Self {
        Self::XMLError(value)
    }
}

impl From<std::string::FromUtf8Error> for EnvoyError {
    fn from(value: std::string::FromUtf8Error) -> Self {
        Self::XMLUtf8Error(value)
    }
}

impl From<tokio::io::Error> for EnvoyError {
    fn from(_: tokio::io::Error) -> Self {
        Self::TCPConnectionError
    }
}

impl std::fmt::Display for EnvoyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RequestError(e) => {
                write!(f, "Envoy recieved an error while making a request: {e}")
            }
            Self::MessageParseError(e) => write!(f, "Envoy failed to parse a message to yaml: {e}"),
            Self::ECCOperationError(e) => write!(f, "Envoy recieved ECC operation error: {e}"),
            Self::FribOperationError(e) => write!(f, "Envoy recieved FRIB operation error: {e}"),
            Self::ECCStatusError(e) => write!(f, "Envoy recieved ECC status error: {e}"),
            Self::FribStatusError(e) => write!(f, "Envoy recieved FRIB status error: {e}"),
            Self::SendError(e) => write!(f, "Envoy failed to send a message: {e}"),
            Self::StringToIntError(e) => write!(f, "Envoy failed to parse string to integer: {e}"),
            Self::StringToFloatError(e) => write!(f, "Envoy failed to parse string to float: {e}"),
            Self::XMLError(e) => write!(f, "Envoy failed to parse XML body: {e}"),
            Self::XMLUtf8Error(e) => write!(f, "Envoy failed to convert XML to String: {e}"),
            Self::XMLConversionError => write!(f, "Envoy failed to convert XML data!"),
            Self::TCPConnectionError => {
                write!(
                    f,
                    "Envoy ran into an error connecting a TCPStream, most likely a timeout"
                )
            }
        }
    }
}

impl std::error::Error for EnvoyError {}

#[derive(Debug)]
pub enum EmbassyError {
    MessageSendError(SendError<EmbassyMessage>),
    MessageKindError(MessageKind, MessageKind),
    MessageParseError(serde_yaml::Error),
    MessageRecieveError,
}

impl From<SendError<EmbassyMessage>> for EmbassyError {
    fn from(value: SendError<EmbassyMessage>) -> Self {
        Self::MessageSendError(value)
    }
}

impl From<serde_yaml::Error> for EmbassyError {
    fn from(value: serde_yaml::Error) -> Self {
        Self::MessageParseError(value)
    }
}

impl std::fmt::Display for EmbassyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MessageKindError(expected, recieved) => write!(
                f,
                "Embassy expected {expected} message, recieved {recieved} message!"
            ),
            Self::MessageSendError(e) => {
                write!(f, "Embassy had an error sending the following message: {e}")
            }
            Self::MessageParseError(e) => write!(f, "Embassy had an error parsing a message: {e}"),
            Self::MessageRecieveError => {
                write!(f, "Embassy communication lines were disconnected!")
            }
        }
    }
}

impl std::error::Error for EmbassyError {}
