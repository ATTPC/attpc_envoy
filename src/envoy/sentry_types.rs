use super::message::{MessageKind, ToMessage};
use serde::{Deserialize, Serialize};
const SENTRY_ONLINE: &str = "Online";
const SENTRY_OFFLINE: &str = "Offline";
const SENTRY_INCONSISTENT: &str = "Inconsistent";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentryParameters {
    pub experiment: String,
    pub run_number: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentryResponse {
    pub disk: String,
    pub process: String,
    pub data_path: String,
    pub data_written_gb: f64,
    pub data_path_files: i32,
    pub disk_avail_gb: f64,
    pub disk_total_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentryStatus {
    pub disk: String,
    pub process: String,
    pub data_path: String,
    pub data_written_gb: f64,
    pub data_path_files: i32,
    pub disk_avail_gb: f64,
    pub disk_total_gb: f64,
    pub data_rate_mb: f64,
}

impl Default for SentryStatus {
    fn default() -> Self {
        Self {
            disk: String::from("N/A"),
            process: String::from("N/A"),
            data_path: String::from("N/A"),
            data_written_gb: 0.0,
            data_path_files: 0,
            disk_avail_gb: 0.0,
            disk_total_gb: 0.0,
            data_rate_mb: 0.0,
        }
    }
}

impl ToMessage for SentryStatus {
    fn message_kind(&self) -> MessageKind {
        MessageKind::SentryStatus
    }
}

impl SentryStatus {
    pub fn from_response(
        resp: SentryResponse,
        prev_written_gb: &f64,
        ellapsed_time_sec: f64,
    ) -> Self {
        Self {
            disk: resp.disk,
            process: resp.process,
            data_path: resp.data_path,
            data_written_gb: prev_written_gb + resp.data_written_gb,
            data_path_files: resp.data_path_files,
            disk_avail_gb: resp.disk_avail_gb,
            disk_total_gb: resp.disk_total_gb,
            data_rate_mb: (resp.data_written_gb / ellapsed_time_sec) * 1.0e3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SentryOperation {
    Catalog(SentryParameters),
    Backup(SentryParameters),
}

impl ToMessage for SentryOperation {
    fn message_kind(&self) -> MessageKind {
        MessageKind::SentryOperation
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SentryServerStatus {
    Online,
    Offline,
    Inconsistent,
}

impl std::fmt::Display for SentryServerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Online => write!(f, "{SENTRY_ONLINE}"),
            Self::Offline => write!(f, "{SENTRY_OFFLINE}"),
            Self::Inconsistent => write!(f, "{SENTRY_INCONSISTENT}"),
        }
    }
}

impl From<&SentryStatus> for SentryServerStatus {
    fn from(value: &SentryStatus) -> Self {
        if value.disk != "N/A" {
            Self::Online
        } else {
            Self::Offline
        }
    }
}

impl From<SentryServerStatus> for String {
    fn from(value: SentryServerStatus) -> Self {
        match value {
            SentryServerStatus::Online => Self::from(SENTRY_ONLINE),
            SentryServerStatus::Offline => Self::from(SENTRY_OFFLINE),
            SentryServerStatus::Inconsistent => Self::from(SENTRY_INCONSISTENT),
        }
    }
}
