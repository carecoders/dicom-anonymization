use crate::hasher::Error as HashingError;
use dicom_core::value::CastValueError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub(crate) enum ActionError {
    #[error("Internal error: {}", .0.to_lowercase())]
    InternalError(String),

    #[error("Invalid input: {}", .0.to_lowercase())]
    InvalidInput(String),

    #[error("Value error: {}", .0.to_lowercase())]
    ValueError(String),
}

impl From<HashingError> for ActionError {
    fn from(err: HashingError) -> Self {
        ActionError::InternalError(format!("{err}"))
    }
}

impl From<CastValueError> for ActionError {
    fn from(err: CastValueError) -> Self {
        ActionError::ValueError(format!("{err}"))
    }
}
