use dicom_core::header::Header;
use dicom_core::value::CastValueError;
use dicom_object::mem::InMemElement;
use dicom_object::{AccessError, DefaultDicomObject};
use log::warn;
use std::borrow::Cow;
use thiserror::Error;

use crate::actions::errors::ActionError;
use crate::config::Config;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("Value error: {}", .0.to_lowercase())]
    ValueError(String),

    #[error("Element error: {}", .0.to_lowercase())]
    ElementError(String),

    #[error("Anonymization error: {}", .0.to_lowercase())]
    AnonymizationError(String),
}

impl From<CastValueError> for Error {
    fn from(err: CastValueError) -> Self {
        Error::ValueError(format!("{err}"))
    }
}

impl From<AccessError> for Error {
    fn from(err: AccessError) -> Self {
        Error::ElementError(format!("{err}"))
    }
}

impl From<ActionError> for Error {
    fn from(err: ActionError) -> Self {
        Error::AnonymizationError(format!("{err}"))
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub trait Processor {
    fn process_element<'a>(
        &'a self,
        obj: &DefaultDicomObject,
        elem: &'a InMemElement,
    ) -> Result<Option<Cow<'a, InMemElement>>>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct DefaultProcessor {
    config: Config,
}

impl DefaultProcessor {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl Processor for DefaultProcessor {
    /// Process a DICOM data element according to the configured anonymization rules
    ///
    /// Takes a DICOM object and one of its elements, applies the appropriate anonymization
    /// action based on the configuration, and returns the result.
    ///
    /// # Arguments
    ///
    /// * `obj` - Reference to the DICOM object containing the element
    /// * `elem` - Reference to the element to be processed
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// * `Some(Cow<InMemElement>)` - The processed element
    /// * `None` - If the element should be removed
    /// * `Err` - If there was an error processing the element
    fn process_element<'a>(
        &'a self,
        obj: &DefaultDicomObject,
        elem: &'a InMemElement,
    ) -> Result<Option<Cow<'a, InMemElement>>> {
        let action = self.config.get_action(&elem.tag());
        let action_struct = action.get_action_struct();

        let process_result = action_struct.process(&self.config, obj, elem);
        match process_result {
            Ok(None) => Ok(None),
            Ok(Some(v)) => Ok(Some(Cow::Owned(v.into_owned()))),
            Err(ActionError::InvalidHashDateTag(e)) => {
                // log a warning for this error, but return the element as is
                warn!("{}", e);
                Ok(Some(Cow::Borrowed(elem)))
            }
            Err(e) => Err(Error::from(e)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct DoNothingProcessor;

impl DoNothingProcessor {
    fn new() -> Self {
        Self {}
    }
}

impl Default for DoNothingProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for DoNothingProcessor {
    fn process_element<'a>(
        &'a self,
        _obj: &DefaultDicomObject,
        elem: &'a InMemElement,
    ) -> Result<Option<Cow<'a, InMemElement>>> {
        // just return it as is, without any changes
        Ok(Some(Cow::Borrowed(elem)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use dicom_core::header::HasLength;
    use dicom_core::value::Value;
    use dicom_core::{header, PrimitiveValue, VR};
    use dicom_dictionary_std::tags;
    use dicom_object::FileDicomObject;

    use crate::actions::Action;
    use crate::config::ConfigBuilder;
    use crate::test_utils::make_file_meta;

    #[test]
    fn test_process_element_hash_length() {
        let meta = make_file_meta();
        let mut obj = FileDicomObject::new_empty_with_meta(meta);

        obj.put(InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::from("0123456789ABCDEF"),
        ));

        let config = ConfigBuilder::new()
            .tag_action(tags::ACCESSION_NUMBER, Action::Hash { length: None })
            .build();

        let elem = obj.element(tags::ACCESSION_NUMBER).unwrap();
        let processor = DefaultProcessor::new(config);
        let processed = processor.process_element(&obj, elem).unwrap();
        assert_eq!(processed.unwrap().value().length(), header::Length(16));
    }

    #[test]
    fn test_process_element_hash_max_length() {
        let meta = make_file_meta();
        let mut obj = FileDicomObject::new_empty_with_meta(meta);

        obj.put(InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::from("0123456789ABCDEF"),
        ));

        let config = ConfigBuilder::new()
            .tag_action(tags::ACCESSION_NUMBER, Action::Hash { length: Some(32) })
            .build();

        let elem = obj.element(tags::ACCESSION_NUMBER).unwrap();
        let processor = DefaultProcessor::new(config);
        let processed = processor.process_element(&obj, elem).unwrap();
        // new value length should have been cut off at the max length for SH VR, which is 16
        assert_eq!(processed.unwrap().value().length(), header::Length(16));
    }

    #[test]
    fn test_process_element_hash_length_with_value() {
        let meta = make_file_meta();
        let mut obj = FileDicomObject::new_empty_with_meta(meta);

        obj.put(InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::from("0123456789ABCDEF"),
        ));

        let config = ConfigBuilder::new()
            .tag_action(tags::ACCESSION_NUMBER, Action::Hash { length: Some(8) })
            .build();

        let elem = obj.element(tags::ACCESSION_NUMBER).unwrap();
        let processor = DefaultProcessor::new(config);
        let processed = processor.process_element(&obj, elem).unwrap();
        assert_eq!(processed.unwrap().value().length(), header::Length(8));
    }

    #[test]
    fn test_process_element_hash_date_invalid_hash_date_tag_error() {
        let meta = make_file_meta();
        let mut obj = FileDicomObject::new_empty_with_meta(meta);

        obj.put(InMemElement::new(
            tags::STUDY_DATE,
            VR::DA,
            Value::from("20010102"),
        ));

        let config = ConfigBuilder::new()
            .tag_action(
                tags::STUDY_DATE,
                Action::HashDate {
                    other_tag: tags::PATIENT_ID,
                },
            )
            .build();

        let elem = obj.element(tags::STUDY_DATE).unwrap();
        let processor = DefaultProcessor::new(config);
        let processed = processor.process_element(&obj, elem).unwrap();

        // element should be returned as is because the `PatientID` tag is not in the DICOM object
        assert_eq!(&processed.unwrap().into_owned(), elem);
    }

    #[test]
    fn test_process_element_replace() {
        let meta = make_file_meta();
        let mut obj = FileDicomObject::new_empty_with_meta(meta);

        obj.put(InMemElement::new(
            tags::PATIENT_NAME,
            VR::PN,
            Value::from("John Doe"),
        ));

        let config = ConfigBuilder::new()
            .tag_action(
                tags::PATIENT_NAME,
                Action::Replace {
                    value: "Jane Doe".into(),
                },
            )
            .build();

        let elem = obj.element(tags::PATIENT_NAME).unwrap();
        let processor = DefaultProcessor::new(config);
        let processed = processor.process_element(&obj, elem).unwrap();
        assert_eq!(processed.unwrap().value(), &Value::from("Jane Doe"));
    }

    #[test]
    fn test_process_element_keep() {
        let meta = make_file_meta();
        let mut obj = FileDicomObject::new_empty_with_meta(meta);

        obj.put(InMemElement::new(
            tags::PATIENT_NAME,
            VR::PN,
            Value::from("John Doe"),
        ));

        let config = ConfigBuilder::new()
            .tag_action(tags::PATIENT_NAME, Action::Keep)
            .build();

        let elem = obj.element(tags::PATIENT_NAME).unwrap();
        let processor = DefaultProcessor::new(config);
        let processed = processor.process_element(&obj, elem).unwrap();
        assert_eq!(&processed.unwrap().into_owned(), elem);
    }

    #[test]
    fn test_process_element_empty() {
        let meta = make_file_meta();
        let mut obj = FileDicomObject::new_empty_with_meta(meta);

        obj.put(InMemElement::new(
            tags::PATIENT_NAME,
            VR::PN,
            Value::from("John Doe"),
        ));

        let config = ConfigBuilder::new()
            .tag_action(tags::PATIENT_NAME, Action::Empty)
            .build();

        let elem = obj.element(tags::PATIENT_NAME).unwrap();
        let processor = DefaultProcessor::new(config);
        let processed = processor.process_element(&obj, elem).unwrap();
        assert_eq!(
            processed.unwrap().value(),
            &Value::Primitive(PrimitiveValue::Empty)
        );
    }

    #[test]
    fn test_process_element_remove() {
        let meta = make_file_meta();
        let mut obj = FileDicomObject::new_empty_with_meta(meta);

        obj.put(InMemElement::new(
            tags::PATIENT_NAME,
            VR::PN,
            Value::from("John Doe"),
        ));

        let config = ConfigBuilder::new()
            .tag_action(tags::PATIENT_NAME, Action::Remove)
            .build();

        let elem = obj.element(tags::PATIENT_NAME).unwrap();
        let processor = DefaultProcessor::new(config);
        let processed = processor.process_element(&obj, elem).unwrap();
        assert_eq!(processed, None);
    }

    #[test]
    fn test_do_nothing_processor() {
        let meta = make_file_meta();
        let mut obj = FileDicomObject::new_empty_with_meta(meta);

        obj.put(InMemElement::new(
            tags::PATIENT_NAME,
            VR::PN,
            Value::from("John Doe"),
        ));

        let elem = obj.element(tags::PATIENT_NAME).unwrap();
        let processor = DoNothingProcessor::new();
        let processed = processor.process_element(&obj, elem).unwrap();
        assert_eq!(processed.unwrap().into_owned(), elem.clone());
    }
}
