//! Anonymizes a DICOM object based on the configured actions.
//!
//! This module provides functionality to anonymize DICOM (Digital Imaging and Communications in Medicine) objects
//! by applying various actions to specific DICOM tags. The anonymization process can remove, empty, or change
//! the content of certain data elements based on the configuration.
//!
//! The main components of this module are:
//! - [`ConfigBuilder`]: Struct for building the configuration.
//! - [`DefaultProcessor`]: Processes the various data elements based on the configuration.
//! - [`Anonymizer`]: The main struct that performs the anonymization process.
//! - [`AnonymizationResult`]: The result of the anonymization process.
//!
//! # Example
//!
//! ```
//! use std::fs::File;
//! use dicom_anonymization::Anonymizer;
//! use dicom_anonymization::config::builder::ConfigBuilder;
//! use dicom_anonymization::processor::DefaultProcessor;
//!
//! let config_builder = ConfigBuilder::default();
//! let config = config_builder.build();
//!
//! let processor = DefaultProcessor::new(config);
//! let anonymizer = Anonymizer::new(processor);
//!
//! let file = File::open("tests/data/test.dcm").unwrap();
//! let result = anonymizer.anonymize(file).unwrap();
//! ```
//!
//! This module is designed to be flexible, allowing users to customize the anonymization process
//! according to their specific requirements and privacy regulations.

pub mod actions;
pub mod config;
mod dicom;
pub mod hasher;
pub mod processor;
mod test_utils;

use std::io::{Read, Write};

use crate::config::builder::ConfigBuilder;
use crate::processor::{DefaultProcessor, Error as ProcessingError};
pub use dicom_core::Tag;
pub use dicom_dictionary_std::tags;
use dicom_object::{DefaultDicomObject, FileDicomObject, OpenFileOptions, ReadError, WriteError};
use processor::Processor;
use thiserror::Error;

/// Represents the result of a DICOM anonymization process.
///
/// This struct contains both the original and anonymized DICOM objects after processing.
/// It allows access to both versions for comparison or verification purposes.
///
/// # Fields
///
/// * `original` - The original, unmodified DICOM object before anonymization
/// * `anonymized` - The resulting DICOM object after anonymization
#[derive(Debug, Clone, PartialEq)]
pub struct AnonymizationResult {
    pub original: DefaultDicomObject,
    pub anonymized: DefaultDicomObject,
}

#[derive(Error, Debug, PartialEq)]
pub enum AnonymizationError {
    #[error("Read error: {}", .0.to_lowercase())]
    ReadError(String),

    #[error("Write error: {}", .0.to_lowercase())]
    WriteError(String),

    #[error("{0}")]
    ProcessingError(String),
}

impl From<ReadError> for AnonymizationError {
    fn from(err: ReadError) -> Self {
        AnonymizationError::ReadError(format!("{err}"))
    }
}

impl From<WriteError> for AnonymizationError {
    fn from(err: WriteError) -> Self {
        AnonymizationError::WriteError(format!("{err}"))
    }
}

impl From<ProcessingError> for AnonymizationError {
    fn from(err: ProcessingError) -> Self {
        AnonymizationError::ProcessingError(format!("{err}"))
    }
}

pub type Result<T, E = AnonymizationError> = std::result::Result<T, E>;

impl AnonymizationResult {
    /// Writes the anonymized DICOM object to the provided writer.
    ///
    /// # Arguments
    ///
    /// * `to` - A writer implementing the `Write` trait where the anonymized DICOM object will be written to.
    ///
    /// # Returns
    ///
    /// Returns a `Result<()>` indicating success or an error if the write operation fails.
    ///
    /// # Example
    ///
    /// ```
    /// use std::fs::File;
    /// use dicom_anonymization::Anonymizer;
    ///
    /// let anonymizer = Anonymizer::default();
    /// let file = File::open("tests/data/test.dcm").unwrap();
    /// let result = anonymizer.anonymize(file).unwrap();
    ///
    /// // output can be a file or anything else that implements the `Write` trait, like this one:
    /// let mut output = Vec::<u8>::new();
    /// result.write(&mut output).unwrap();
    /// ```
    pub fn write<W: Write>(&self, to: W) -> Result<()> {
        self.anonymized.write_all(to)?;
        Ok(())
    }
}

/// A struct for performing the anonymization process on DICOM objects.
///
/// The [`Anonymizer`] contains a `Box<dyn Processor>` which performs the actual anonymization by applying
/// processor-defined transformations to DICOM data elements. The processor must implement both the `Processor`
/// trait and be `Sync`.
pub struct Anonymizer {
    processor: Box<dyn Processor + Send + Sync>,
}

impl Anonymizer {
    pub fn new<T>(processor: T) -> Self
    where
        T: Processor + Send + Sync + 'static,
    {
        Self {
            processor: Box::new(processor),
        }
    }

    /// Performs the anonymization process on the given DICOM object.
    ///
    /// This function takes a source implementing the `Read` trait and returns an [`AnonymizationResult`]
    /// containing both the original and anonymized DICOM objects.
    ///
    /// # Arguments
    ///
    /// * `src` - A source implementing the `Read` trait containing a DICOM object
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the [`AnonymizationResult`] if successful, or an
    /// [`AnonymizationError`] if the anonymization process fails in some way.
    ///
    /// # Example
    ///
    /// ```
    /// use std::fs::File;
    /// use dicom_anonymization::Anonymizer;
    ///
    /// let anonymizer = Anonymizer::default();
    /// let file = File::open("tests/data/test.dcm").unwrap();
    /// let result = anonymizer.anonymize(file).unwrap();
    /// ```
    pub fn anonymize(&self, src: impl Read) -> Result<AnonymizationResult> {
        let obj = OpenFileOptions::new().from_reader(src)?;
        let mut new_obj = FileDicomObject::new_empty_with_meta(obj.meta().clone());

        for elem in &obj {
            let result = self.processor.process_element(&obj, elem);
            match result {
                Ok(None) => continue,
                Ok(Some(processed_elem)) => {
                    new_obj.put(processed_elem.into_owned());
                }
                Err(err) => return Err(err.into()),
            }
        }

        // Make `MediaStorageSOPInstanceUID` the same as `SOPInstanceUID`
        if let Ok(elem) = new_obj.element(tags::SOP_INSTANCE_UID) {
            let sop_instance_uid = elem.value().clone();
            let meta = new_obj.meta_mut();
            if let Ok(sop_instance_uid_str) = sop_instance_uid.to_str() {
                meta.media_storage_sop_instance_uid = sop_instance_uid_str.into_owned();
                meta.update_information_group_length();
            }
        }

        // Make `MediaStorageSOPClassUID` the same as `SOPClassUID`
        if let Ok(elem) = new_obj.element(tags::SOP_CLASS_UID) {
            let sop_class_uid = elem.value().clone();
            let meta = new_obj.meta_mut();
            if let Ok(sop_class_uid_str) = sop_class_uid.to_str() {
                meta.media_storage_sop_class_uid = sop_class_uid_str.into_owned();
                meta.update_information_group_length();
            }
        }

        Ok(AnonymizationResult {
            original: obj,
            anonymized: new_obj,
        })
    }
}

impl Default for Anonymizer {
    /// Returns a default instance of [`Anonymizer`] with standard anonymization settings.
    ///
    /// This creates an [`Anonymizer`] with a [`DefaultProcessor`] that uses the default
    /// configuration from the [`ConfigBuilder`].
    ///
    /// # Returns
    ///
    /// A new [`Anonymizer`] instance with default settings.
    ///
    /// # Example
    ///
    /// ```
    /// use dicom_anonymization::Anonymizer;
    ///
    /// let anonymizer = Anonymizer::default();
    /// ```
    fn default() -> Self {
        let config = ConfigBuilder::default().build();
        let processor = DefaultProcessor::new(config);
        Self::new(processor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tags;
    use dicom_core::value::Value;
    use dicom_core::{PrimitiveValue, VR};
    use dicom_object::mem::InMemElement;
    use dicom_object::InMemDicomObject;

    use crate::config::builder::ConfigBuilder;
    use crate::processor::DefaultProcessor;
    use crate::test_utils::make_file_meta;
    use crate::Tag;

    #[test]
    fn test_anonymizer() {
        let meta = make_file_meta();
        let mut obj: FileDicomObject<InMemDicomObject> = FileDicomObject::new_empty_with_meta(meta);

        obj.put(InMemElement::new(
            tags::PATIENT_NAME,
            VR::PN,
            Value::from("John Doe"),
        ));

        obj.put(InMemElement::new(
            tags::PATIENT_ID,
            VR::LO,
            Value::from("12345"),
        ));

        obj.put(InMemElement::new(
            Tag::from([0x0033, 0x1010]),
            VR::LO,
            Value::from("I am a private tag and should be removed"),
        ));

        let mut file = Vec::new();
        obj.write_all(&mut file).unwrap();

        let config = ConfigBuilder::default().build();
        let processor = DefaultProcessor::new(config);
        let anonymizer = Anonymizer::new(processor);
        let result = anonymizer.anonymize(file.as_slice()).unwrap();

        assert!(result.anonymized.element(tags::PATIENT_NAME).is_ok());
        assert_eq!(
            result
                .anonymized
                .element(tags::PATIENT_NAME)
                .unwrap()
                .value(),
            &Value::Primitive(PrimitiveValue::Str("6652061665".to_string()))
        );

        assert!(result.anonymized.element(tags::PATIENT_ID).is_ok());
        assert_eq!(
            result.anonymized.element(tags::PATIENT_ID).unwrap().value(),
            &Value::Primitive(PrimitiveValue::from("6662505961"))
        );

        // private tag should be removed after anonymization
        assert!(result.original.element(Tag::from([0x0033, 0x1010])).is_ok());
        assert!(result
            .anonymized
            .element(Tag::from([0x0033, 0x1010]))
            .is_err());
    }
}
