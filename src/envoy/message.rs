use super::ecc_envoy::{ECCOperationResponse, ECCStatusResponse};
use super::error::EmbassyError;
use super::sentry_types::SentryStatus;
use serde::Serialize;

/// Types of messages the Embassy might recieve
#[derive(Debug, Clone, PartialEq)]
pub enum MessageKind {
    ECCOperation,
    ECCOpResponse,
    ECCStatus,
    SentryOperation,
    SentryStatus,
    Cancel,
}

impl std::fmt::Display for MessageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ECCOperation => write!(f, "ECCOperation"),
            Self::ECCOpResponse => write!(f, "ECCOpResponse"),
            Self::ECCStatus => write!(f, "ECCStatus"),
            Self::SentryOperation => write!(f, "SentryOperation"),
            Self::SentryStatus => write!(f, "SentryStatus"),
            Self::Cancel => write!(f, "Cancel"),
        }
    }
}

pub trait ToMessage {
    fn message_kind(&self) -> MessageKind;
}

/// A unified message type to simplify the transfer of data from the various types of envoys to
/// the embassy and the embassy to the UI.
/// Typically the data contained is some form of xml, json, or yaml string. Can be cast to specific message
/// types using the TryFrom trait.
#[derive(Debug, Clone)]
pub struct EmbassyMessage {
    pub kind: MessageKind,
    pub id: usize,
    pub body: String,
}

impl std::fmt::Display for EmbassyMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EmbassyMessage from {} of kind {} with body: {}",
            self.id, self.kind, self.body
        )
    }
}

impl EmbassyMessage {
    pub fn compose(item: impl ToMessage + Serialize, id: usize) -> Self {
        Self {
            kind: item.message_kind(),
            id,
            body: serde_json::to_string(&item).expect("Serializing failed somehow..."),
        }
    }

    pub fn compose_cancel() -> Self {
        Self {
            kind: MessageKind::Cancel,
            id: 0,
            body: String::from("Cancel"),
        }
    }
}

impl TryInto<ECCStatusResponse> for EmbassyMessage {
    type Error = EmbassyError;
    fn try_into(self) -> Result<ECCStatusResponse, Self::Error> {
        match self.kind {
            MessageKind::ECCStatus => Ok(serde_json::from_str::<ECCStatusResponse>(&self.body)?),
            _ => Err(Self::Error::InvalidKind(MessageKind::ECCStatus, self.kind)),
        }
    }
}

impl TryInto<ECCStatusResponse> for &EmbassyMessage {
    type Error = EmbassyError;
    fn try_into(self) -> Result<ECCStatusResponse, Self::Error> {
        match self.kind {
            MessageKind::ECCStatus => Ok(serde_json::from_str::<ECCStatusResponse>(&self.body)?),
            _ => Err(Self::Error::InvalidKind(
                MessageKind::ECCStatus,
                self.kind.clone(),
            )),
        }
    }
}

impl TryInto<ECCOperationResponse> for EmbassyMessage {
    type Error = EmbassyError;
    fn try_into(self) -> Result<ECCOperationResponse, Self::Error> {
        match self.kind {
            MessageKind::ECCOpResponse => {
                Ok(serde_json::from_str::<ECCOperationResponse>(&self.body)?)
            }
            _ => Err(Self::Error::InvalidKind(
                MessageKind::ECCOperation,
                self.kind,
            )),
        }
    }
}

impl TryInto<ECCOperationResponse> for &EmbassyMessage {
    type Error = EmbassyError;
    fn try_into(self) -> Result<ECCOperationResponse, Self::Error> {
        match self.kind {
            MessageKind::ECCOpResponse => {
                Ok(serde_json::from_str::<ECCOperationResponse>(&self.body)?)
            }
            _ => Err(Self::Error::InvalidKind(
                MessageKind::ECCOperation,
                self.kind.clone(),
            )),
        }
    }
}

impl TryInto<SentryStatus> for EmbassyMessage {
    type Error = EmbassyError;
    fn try_into(self) -> Result<SentryStatus, Self::Error> {
        match self.kind {
            MessageKind::SentryStatus => Ok(serde_json::from_str::<SentryStatus>(&self.body)?),
            _ => Err(Self::Error::InvalidKind(MessageKind::ECCStatus, self.kind)),
        }
    }
}

impl TryInto<SentryStatus> for &EmbassyMessage {
    type Error = EmbassyError;
    fn try_into(self) -> Result<SentryStatus, Self::Error> {
        match self.kind {
            MessageKind::SentryStatus => Ok(serde_json::from_str::<SentryStatus>(&self.body)?),
            _ => Err(Self::Error::InvalidKind(
                MessageKind::ECCStatus,
                self.kind.clone(),
            )),
        }
    }
}
