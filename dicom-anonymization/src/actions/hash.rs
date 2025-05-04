use dicom_core::header::Header;
use dicom_core::{DataElement, PrimitiveValue};
use dicom_object::mem::InMemElement;
use dicom_object::DefaultDicomObject;
use std::borrow::Cow;
use thiserror::Error;

use crate::actions::errors::ActionError;
use crate::actions::utils::{is_empty_element, truncate_to};
use crate::actions::DataElementAction;
use crate::config::{Config, ConfigError};
use crate::dicom;
use crate::hasher::HashFn;

pub const HASH_LENGTH_MINIMUM: usize = 8;

#[derive(Error, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[error("{0}")]
pub struct HashLengthError(String);

/// A newtype wrapper for specifying the length of a hash value.
/// The internal value represents the number of characters the hash should be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HashLength(pub usize);

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

#[derive(Debug, Clone, PartialEq)]
pub struct Hash {
    length: Option<HashLength>,
}

impl Hash {
    pub fn new(length: Option<HashLength>) -> Self {
        Self { length }
    }

    fn anonymize(
        &self,
        hash_fn: HashFn,
        value: &str,
        max_length: Option<usize>,
    ) -> Result<String, ActionError> {
        let anonymized_value = hash_fn(value)?;

        let length = match self.length {
            Some(length) => match max_length {
                Some(max_length) if max_length < length.0 => Some(HashLength(max_length)),
                _ => Some(HashLength(length.0)),
            },
            None => max_length.map(HashLength),
        };

        let result = match length {
            Some(length) => truncate_to(length.0, &anonymized_value.to_string()),
            None => anonymized_value.to_string(),
        };

        Ok(result)
    }
}

impl Default for Hash {
    fn default() -> Self {
        Self::new(None)
    }
}

impl DataElementAction for Hash {
    fn process<'a>(
        &'a self,
        config: &Config,
        _obj: &DefaultDicomObject,
        elem: &'a InMemElement,
    ) -> Result<Option<Cow<'a, InMemElement>>, ActionError> {
        if is_empty_element(elem) {
            return Ok(Some(Cow::Borrowed(elem)));
        }

        let hash_fn = config.get_hash_fn();
        let max_length = dicom::max_length_for_vr(elem.vr());
        let elem_value = elem.value().string()?;
        let anonymized_value = self.anonymize(hash_fn, elem_value, max_length)?;

        let new_elem = DataElement::new::<PrimitiveValue>(
            elem.tag(),
            elem.vr(),
            PrimitiveValue::from(anonymized_value),
        );
        Ok(Some(Cow::Owned(new_elem)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use dicom_core::header::HasLength;
    use dicom_core::value::Value;
    use dicom_core::{header, VR};
    use dicom_object::FileDicomObject;

    use crate::hasher::blake3_hash_fn;
    use crate::tags;
    use crate::test_utils::make_file_meta;

    #[test]
    fn test_process_without_length() {
        let mut obj = FileDicomObject::new_empty_with_meta(make_file_meta());
        let elem = InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::from("0123456789ABCDEF"),
        );
        obj.put(elem.clone());

        let action_struct = Hash::default();
        let config = Config::default();
        let processed = action_struct.process(&config, &obj, &elem).unwrap();

        // no length given, so 16 should been used for length, because that is the maximum length for VR `SH`
        assert_eq!(processed.unwrap().value().length(), header::Length(16));
    }

    #[test]
    fn test_process_with_length() {
        let mut obj = FileDicomObject::new_empty_with_meta(make_file_meta());
        let elem = InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::from("0123456789ABCDEF"),
        );
        obj.put(elem.clone());

        let action_struct = Hash::new(Some(HashLength(10)));
        let config = Config::default();
        let processed = action_struct.process(&config, &obj, &elem).unwrap();
        assert_eq!(processed.unwrap().value().length(), header::Length(10));
    }

    #[test]
    fn test_process_with_length_exceeding_max_length() {
        let mut obj = FileDicomObject::new_empty_with_meta(make_file_meta());
        let elem = InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::from("0123456789ABCDEF"),
        );
        obj.put(elem.clone());

        let action_struct = Hash::new(Some(HashLength(32)));
        let config = Config::default();
        let processed = action_struct.process(&config, &obj, &elem).unwrap();
        assert_eq!(processed.unwrap().value().length(), header::Length(16));
    }

    #[test]
    fn test_process_empty_element() {
        let mut obj = FileDicomObject::new_empty_with_meta(make_file_meta());
        let elem = InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::Primitive(PrimitiveValue::Empty),
        );
        obj.put(elem.clone());

        let action_struct = Hash::new(Some(HashLength(8)));
        let config = Config::default();
        let processed = action_struct.process(&config, &obj, &elem).unwrap();
        assert_eq!(processed.unwrap().into_owned(), elem);
    }

    #[test]
    fn test_anonymize_no_length() {
        let value = "203087";
        let hash_fn = blake3_hash_fn;
        let action_struct = Hash::default();
        let result = action_struct.anonymize(hash_fn, value, None);
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_anonymize_with_length() {
        let value = "203087";
        let hash_fn = blake3_hash_fn;
        let action_struct = Hash::new(Some(HashLength(10)));
        let result = action_struct.anonymize(hash_fn, value, None);
        assert_eq!(result.unwrap().len(), 10);
    }

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
