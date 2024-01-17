const SURVEYOR_ONLINE_STATE_TEXT: &str = "Online";
const SURVEYOR_OFFLINE_STATE_TEXT: &str = "Offline";
const SURVEYOR_INVALID_STATE_TEXT: &str = "Invalid";
const SURVEYOR_INCONSISTENT_STATE_TEXT: &str = "Invalid";

const SURVEYOR_DISK_FILLED_TEXT: &str = "Filled";
const SURVEYOR_DISK_EMPTY_TEXT: &str = "Empty";
const SURVEYOR_DISK_NA_TEXT: &str = "N/A";

/// The status of the Surveyor. Can be converted to integer.
#[derive(Debug, Clone)]
pub enum SurveyorStatus {
    Online,
    Offline,
    Invalid,
    Inconsistent,
}

impl From<i32> for SurveyorStatus {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Offline,
            1 => Self::Online,
            _ => Self::Invalid,
        }
    }
}

impl std::fmt::Display for SurveyorStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Online => write!(f, "{SURVEYOR_ONLINE_STATE_TEXT}"),
            Self::Offline => write!(f, "{SURVEYOR_OFFLINE_STATE_TEXT}"),
            Self::Invalid => write!(f, "{SURVEYOR_INVALID_STATE_TEXT}"),
            Self::Inconsistent => write!(f, "{SURVEYOR_INCONSISTENT_STATE_TEXT}"),
        }
    }
}

/// The status of the disk being written to by the DataRouter/Surveyor
#[derive(Debug, Clone)]
pub enum SurveyorDiskStatus {
    Filled,
    Empty,
    NA,
}

impl From<&str> for SurveyorDiskStatus {
    fn from(value: &str) -> Self {
        match value {
            SURVEYOR_DISK_FILLED_TEXT => Self::Filled,
            SURVEYOR_DISK_EMPTY_TEXT => Self::Empty,
            _ => Self::NA,
        }
    }
}

impl std::fmt::Display for SurveyorDiskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Filled => write!(f, "{SURVEYOR_DISK_FILLED_TEXT}"),
            Self::Empty => write!(f, "{SURVEYOR_DISK_EMPTY_TEXT}"),
            Self::NA => write!(f, "{SURVEYOR_DISK_NA_TEXT}"),
        }
    }
}
