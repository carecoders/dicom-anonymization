use crate::hasher::Error as HashingError;
use dicom_core::value::CastValueError;
use std::num::ParseIntError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub(crate) enum ActionError {
    #[error("{}", .0.to_lowercase())]
    InternalError(String),

    #[error("{}", .0.to_lowercase())]
    InvalidInput(String),

    #[error("{}", .0.to_lowercase())]
    InvalidHashDateTag(String),

    #[error("{}", .0.to_lowercase())]
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

impl From<ParseIntError> for ActionError {
    fn from(err: ParseIntError) -> Self {
        ActionError::InternalError(format!("{err}"))
    }
}
