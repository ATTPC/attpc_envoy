use super::error::{ECCOperationError, ECCStatusError};

const ECC_OFFLINE_STATUS: &str = "Offline";
const ECC_BUSY_STATUS: &str = "Busy";
const ECC_IDLE_STATUS: &str = "Idle";
const ECC_PREPARED_STATUS: &str = "Prepared";
const ECC_DESCRIBED_STATUS: &str = "Described";
const ECC_READY_STATUS: &str = "Ready";
const ECC_RUNNING_STATUS: &str = "Running";
const ECC_INCONSISTENT_STATUS: &str = "Inconsistent";
const ECC_ERROR_STATUS: &str = "Error";

const ECC_DESCRIBE_OP: &str = "Describe";
const ECC_PREPARE_OP: &str = "Prepare";
const ECC_CONFIGURE_OP: &str = "Configure";
const ECC_START_OP: &str = "Start";
const ECC_UNDO_OP: &str = "Undo";
const ECC_BREAKUP_OP: &str = "Breakup";
const ECC_STOP_OP: &str = "Stop";
const ECC_INVALID_OP: &str = "Invalid";

/// The status of an getECCServer
/// Can be converted to a String or integer
#[derive(Debug, PartialEq, Clone)]
pub enum ECCStatus {
    Offline,
    Busy,
    Idle,
    Prepared,
    Described,
    Ready,
    Running,
    ErrorStat,
    Inconsistent,
}

impl std::fmt::Display for ECCStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Offline => write!(f, "{ECC_OFFLINE_STATUS}"),
            Self::Busy => write!(f, "{ECC_BUSY_STATUS}"),
            Self::Idle => write!(f, "{ECC_IDLE_STATUS}"),
            Self::Prepared => write!(f, "{ECC_PREPARED_STATUS}"),
            Self::Described => write!(f, "{ECC_DESCRIBED_STATUS}"),
            Self::Ready => write!(f, "{ECC_READY_STATUS}"),
            Self::Running => write!(f, "{ECC_RUNNING_STATUS}"),
            Self::ErrorStat => write!(f, "{ECC_ERROR_STATUS}"),
            Self::Inconsistent => write!(f, "{ECC_INCONSISTENT_STATUS}"),
        }
    }
}

impl From<ECCStatus> for String {
    fn from(value: ECCStatus) -> String {
        String::from(match value {
            ECCStatus::Offline => ECC_OFFLINE_STATUS,
            ECCStatus::Busy => ECC_BUSY_STATUS,
            ECCStatus::Idle => ECC_IDLE_STATUS,
            ECCStatus::Prepared => ECC_PREPARED_STATUS,
            ECCStatus::Described => ECC_DESCRIBED_STATUS,
            ECCStatus::Ready => ECC_READY_STATUS,
            ECCStatus::Running => ECC_RUNNING_STATUS,
            ECCStatus::ErrorStat => ECC_ERROR_STATUS,
            ECCStatus::Inconsistent => ECC_INCONSISTENT_STATUS,
        })
    }
}

impl From<ECCStatus> for i32 {
    fn from(value: ECCStatus) -> i32 {
        match value {
            ECCStatus::Offline => 0,
            ECCStatus::Idle => 1,
            ECCStatus::Prepared => 2,
            ECCStatus::Described => 3,
            ECCStatus::Ready => 4,
            ECCStatus::Running => 5,
            ECCStatus::Busy => 6,
            _ => -1,
        }
    }
}

impl TryFrom<String> for ECCStatus {
    type Error = ECCStatusError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            ECC_OFFLINE_STATUS => Ok(Self::Offline),
            ECC_BUSY_STATUS => Ok(Self::Busy),
            ECC_IDLE_STATUS => Ok(Self::Idle),
            ECC_PREPARED_STATUS => Ok(Self::Prepared),
            ECC_DESCRIBED_STATUS => Ok(Self::Described),
            ECC_READY_STATUS => Ok(Self::Ready),
            ECC_RUNNING_STATUS => Ok(Self::Running),
            ECC_ERROR_STATUS => Ok(Self::ErrorStat),
            ECC_INCONSISTENT_STATUS => Ok(Self::Inconsistent),
            _ => Err(Self::Error::BadString(value)),
        }
    }
}

impl From<i32> for ECCStatus {
    fn from(value: i32) -> Self {
        match value {
            0 => ECCStatus::Offline,
            1 => ECCStatus::Idle,
            2 => ECCStatus::Described,
            3 => ECCStatus::Prepared,
            4 => ECCStatus::Ready,
            5 => ECCStatus::Running,
            6 => ECCStatus::Busy,
            _ => ECCStatus::ErrorStat,
        }
    }
}

impl ECCStatus {
    /// Get the operation associated with progressing the system from this
    /// status
    pub fn get_forward_operation(&self) -> ECCOperation {
        match self {
            ECCStatus::Idle => ECCOperation::Describe,
            ECCStatus::Described => ECCOperation::Prepare,
            ECCStatus::Prepared => ECCOperation::Configure,
            _ => ECCOperation::Invalid,
        }
    }

    /// Get the operation associated with regressing the system frmo this
    /// status
    pub fn get_backward_operation(&self) -> ECCOperation {
        match self {
            ECCStatus::Ready => ECCOperation::Breakup,
            ECCStatus::Prepared => ECCOperation::Undo,
            ECCStatus::Described => ECCOperation::Undo,
            _ => ECCOperation::Invalid,
        }
    }

    /// Can the system with this status progress
    pub fn can_go_forward(&self) -> bool {
        matches!(
            self,
            ECCStatus::Idle | ECCStatus::Described | ECCStatus::Prepared
        )
    }

    /// Can the system with this status regress
    pub fn can_go_backward(&self) -> bool {
        matches!(
            self,
            ECCStatus::Ready | ECCStatus::Prepared | ECCStatus::Described
        )
    }
}

/// An operation to be performed on
/// a getECCServer. Can be converted to String.
#[derive(Debug, Clone)]
pub enum ECCOperation {
    Describe,
    Prepare,
    Configure,
    Start,
    Undo,
    Breakup,
    Stop,
    Invalid,
}

impl std::fmt::Display for ECCOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Describe => write!(f, "{ECC_DESCRIBE_OP}"),
            Self::Prepare => write!(f, "{ECC_PREPARE_OP}"),
            Self::Configure => write!(f, "{ECC_CONFIGURE_OP}"),
            Self::Start => write!(f, "{ECC_START_OP}"),
            Self::Undo => write!(f, "{ECC_UNDO_OP}"),
            Self::Breakup => write!(f, "{ECC_BREAKUP_OP}"),
            Self::Stop => write!(f, "{ECC_STOP_OP}"),
            Self::Invalid => write!(f, "{ECC_INVALID_OP}"),
        }
    }
}

impl TryFrom<String> for ECCOperation {
    type Error = ECCOperationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            ECC_DESCRIBE_OP => Ok(Self::Describe),
            ECC_PREPARE_OP => Ok(Self::Prepare),
            ECC_CONFIGURE_OP => Ok(Self::Configure),
            ECC_START_OP => Ok(Self::Start),
            ECC_UNDO_OP => Ok(Self::Undo),
            ECC_BREAKUP_OP => Ok(Self::Breakup),
            ECC_STOP_OP => Ok(Self::Stop),
            ECC_INVALID_OP => Ok(Self::Invalid),
            _ => Err(Self::Error::BadString(value)),
        }
    }
}

impl From<ECCOperation> for String {
    fn from(value: ECCOperation) -> String {
        String::from(match value {
            ECCOperation::Describe => ECC_DESCRIBE_OP,
            ECCOperation::Prepare => ECC_PREPARE_OP,
            ECCOperation::Configure => ECC_CONFIGURE_OP,
            ECCOperation::Start => ECC_START_OP,
            ECCOperation::Undo => ECC_UNDO_OP,
            ECCOperation::Breakup => ECC_BREAKUP_OP,
            ECCOperation::Stop => ECC_STOP_OP,
            ECCOperation::Invalid => ECC_INVALID_OP,
        })
    }
}
