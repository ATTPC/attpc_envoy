use super::error::{FribOperationError, FribStatusError};

const FRIB_BEGIN_RUN: &str = "begin";
const FRIB_END_RUN: &str = "end";
const FRIB_CHECK_STATUS: &str = "get state";

const FRIB_OK: &str = "OK";
const FRIB_ERROR: &str = "ERROR";
const FRIB_FAIL: &str = "FAIL";

#[derive(Debug, Clone)]
pub enum FribOperation {
    Begin,
    End,
    Check,
}

impl TryFrom<String> for FribOperation {
    type Error = FribOperationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            FRIB_BEGIN_RUN => Ok(Self::Begin),
            FRIB_END_RUN => Ok(Self::End),
            FRIB_CHECK_STATUS => Ok(Self::Check),
            _ => Err(Self::Error::BadString(value)),
        }
    }
}

impl ToString for FribOperation {
    fn to_string(&self) -> String {
        match self {
            Self::Begin => String::from(FRIB_BEGIN_RUN),
            Self::End => String::from(FRIB_END_RUN),
            Self::Check => String::from(FRIB_CHECK_STATUS),
        }
    }
}

#[derive(Debug, Clone)]
pub enum FribStatus {
    Ok,
    Errored,
    Failed,
}

impl TryFrom<String> for FribStatus {
    type Error = FribStatusError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            FRIB_OK => Ok(Self::Ok),
            FRIB_ERROR => Ok(Self::Errored),
            FRIB_FAIL => Ok(Self::Failed),
            _ => Err(Self::Error::BadString(value)),
        }
    }
}

impl ToString for FribStatus {
    fn to_string(&self) -> String {
        match self {
            Self::Ok => String::from(FRIB_OK),
            Self::Errored => String::from(FRIB_ERROR),
            Self::Failed => String::from(FRIB_FAIL),
        }
    }
}
