use crate::config::ConfigError;
use dicom_core::Tag;
use thiserror::Error;

const HASH_LENGTH_MINIMUM: usize = 8;

#[derive(Error, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[error("{0}")]
pub struct HashLengthError(String);

/// A newtype wrapper for specifying the length of a hash value.
/// The internal value represents the number of characters the hash should be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HashLength(pub(crate) usize);

impl HashLength {
    /// Creates a new [`HashLength`] instance.
    ///
    /// # Arguments
    /// * `length` - The desired length of the hash in characters
    ///
    /// # Returns
    /// * `Ok(HashLength)` if length is valid (>= `HASH_LENGTH_MINIMUM`, which is `8`)
    /// * `Err(HashLengthError)` if length is too short
    pub fn new(length: usize) -> Result<Self, HashLengthError> {
        if length < HASH_LENGTH_MINIMUM {
            return Err(HashLengthError(format!(
                "hash length must be at least {}",
                HASH_LENGTH_MINIMUM
            )));
        }
        Ok(HashLength(length))
    }
}

impl From<HashLengthError> for ConfigError {
    fn from(err: HashLengthError) -> Self {
        ConfigError::InvalidHashLength(err.0)
    }
}

impl TryFrom<usize> for HashLength {
    type Error = HashLengthError;

    fn try_from(value: usize) -> Result<Self, HashLengthError> {
        let hash_length = HashLength::new(value)?;
        Ok(hash_length)
    }
}

/// Specifies the action to perform on DICOM data elements during processing.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Clear the value of the data element.
    Empty,

    /// Completely remove the data element from the DICOM dataset.
    Remove,

    /// Replace the data element value with the specified string.
    Replace(String),

    /// Hash the data element value using an optional custom hash length.
    Hash(Option<HashLength>),

    /// Change a date, using a hash of the given tag value to determine the offset.
    HashDate(Tag),

    /// Generate a new unique identifier (UID) by hashing the original UID.
    HashUID,

    /// Preserve the original data element value without modification.
    Keep,

    /// No action specified.
    None,
}

#[cfg(test)]
mod tests {
    use crate::actions::HashLength;

    #[test]
    fn test_hash_length() {
        assert_eq!(HashLength::new(9).unwrap().0, 9);
    }

    #[test]
    fn test_hash_length_new() {
        assert!(HashLength::new(9).is_ok());
        assert!(HashLength::new(8).is_ok());
        assert!(HashLength::new(7).is_err());
    }

    #[test]
    fn test_hash_length_try_into() {
        assert!(<usize as TryInto<HashLength>>::try_into(9).is_ok());
        assert!(<usize as TryInto<HashLength>>::try_into(8).is_ok());
        assert!(<usize as TryInto<HashLength>>::try_into(7).is_err());
    }

    #[test]
    fn test_hash_length_error() {
        let result = HashLength::new(7);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.to_string(), "hash length must be at least 8");
    }
}
