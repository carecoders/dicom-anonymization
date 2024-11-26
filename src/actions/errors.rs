use crate::hasher::Error as HashingError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub(crate) enum AnonymizeError {
    #[error("Internal error: {}", .0.to_lowercase())]
    InternalError(String),

    #[error("Invalid input: {}", .0.to_lowercase())]
    InvalidInput(String),
}

impl From<HashingError> for AnonymizeError {
    fn from(err: HashingError) -> Self {
        AnonymizeError::InternalError(format!("{err}"))
    }
}
