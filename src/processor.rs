use crate::actions::{Action, HashLength};
use crate::config::{Config, UidRoot};
use crate::functions::anonymize::Anonymize;
use crate::functions::date::HashDateAnonymizer;
use crate::functions::errors::AnonymizeError;
use crate::functions::hash::HashAnonymizer;
use crate::functions::uid::UidAnonymizer;
use crate::hashing::{Blake3Hasher, Hasher};
use crate::vr;
use dicom_core::header::Header;
use dicom_core::value::{CastValueError, Value};
use dicom_core::{DataElement, PrimitiveValue};
use dicom_object::mem::InMemElement;
use dicom_object::{AccessError, DefaultDicomObject};
use log::warn;
use std::borrow::Cow;
use thiserror::Error;

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

impl From<AnonymizeError> for Error {
    fn from(err: AnonymizeError) -> Self {
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

/// A processor for DICOM data elements that applies anonymization rules based on the given configuration
///
/// This processor uses a [`Config`] to determine how to transform individual DICOM elements
/// according to defined anonymization actions like hashing, replacing, or emptying tag values,
/// or completely removing tags.
///
/// Limitation: only top-level DICOM tags are processed for now, not tags nested inside sequences.
/// This may change in the future.
#[derive(Debug, Clone, PartialEq)]
pub struct DataElementProcessor {
    config: Config,
}

impl DataElementProcessor {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl Processor for DataElementProcessor {
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
    /// * `Some(Cow<InMemElement>)` - The processed element, either borrowed or owned
    /// * `None` - If the element should be removed
    /// * `Err` - If there was an error processing the element
    fn process_element<'a>(
        &'a self,
        obj: &DefaultDicomObject,
        elem: &'a InMemElement,
    ) -> Result<Option<Cow<'a, InMemElement>>> {
        let hasher = Blake3Hasher::new();

        match self.config.get_action(&elem.tag()) {
            Action::Empty => process_action_empty(elem),
            Action::Remove => Ok(None),
            Action::Replace(new_value) => process_change_action_replace(elem, new_value),
            Action::Hash(hash_length) => process_change_action_hash(elem, &hasher, *hash_length),
            Action::HashDate(other_tag) => match obj.element(*other_tag) {
                Ok(other_elem) => {
                    if let Ok(other_value) = other_elem.value().string() {
                        process_change_action_hash_date(elem, &hasher, other_value.into())
                    } else {
                        warn!(
                            "did not change tag {} because the other tag {} does not have a valid value",
                            elem.tag(),
                            other_tag
                        );
                        Ok(Some(Cow::Borrowed(elem)))
                    }
                }
                Err(_) => {
                    warn!(
                        "did not change tag {} because the other tag {} is not available",
                        elem.tag(),
                        other_tag
                    );
                    Ok(Some(Cow::Borrowed(elem)))
                }
            },
            Action::HashUID => {
                process_change_action_hash_uid(elem, &hasher, self.config.get_uid_root())
            }
            Action::Keep | Action::None => Ok(Some(Cow::Borrowed(elem))),
        }
    }
}

fn is_empty_element(elem: &InMemElement) -> bool {
    elem.value() == &Value::Primitive(PrimitiveValue::Empty)
}

fn process_change_action_replace<'a>(
    elem: &'a InMemElement,
    new_value: &'a str,
) -> Result<Option<Cow<'a, InMemElement>>> {
    let mut elem = elem.clone();
    elem.update_value(|v| {
        if let Value::Primitive(p) = v {
            *p = PrimitiveValue::Str(new_value.into());
        }
    });
    Ok(Some(Cow::Owned(elem)))
}

fn process_change_action_hash<'a, H: Hasher>(
    elem: &'a InMemElement,
    hasher: &H,
    hash_length: Option<HashLength>,
) -> Result<Option<Cow<'a, InMemElement>>> {
    if is_empty_element(elem) {
        return Ok(Some(Cow::Borrowed(elem)));
    }

    let max_length = vr::max_length(elem.vr());
    let length = match hash_length {
        Some(length) => match max_length {
            Some(max_length) if max_length < length.0 => Some(HashLength(max_length)),
            _ => Some(HashLength(length.0)),
        },
        None => max_length.map(HashLength),
    };

    let value_anonymizer = HashAnonymizer::new(hasher, length);
    let elem_value_as_string = elem.value().string()?;
    let anonymized_value = value_anonymizer.anonymize(elem_value_as_string)?;

    let new_elem = DataElement::new::<PrimitiveValue>(
        elem.tag(),
        elem.vr(),
        PrimitiveValue::from(anonymized_value),
    );
    Ok(Some(Cow::Owned(new_elem)))
}

fn process_change_action_hash_uid<'a, H: Hasher>(
    elem: &'a InMemElement,
    hasher: &H,
    uid_root: &'a UidRoot,
) -> Result<Option<Cow<'a, InMemElement>>> {
    if is_empty_element(elem) {
        return Ok(Some(Cow::Borrowed(elem)));
    }

    let value_anonymizer = UidAnonymizer::new(hasher, uid_root);
    let elem_value_as_string = elem.value().string()?;
    let anonymized_value = value_anonymizer.anonymize(elem_value_as_string)?;

    let new_elem = DataElement::new::<PrimitiveValue>(
        elem.tag(),
        elem.vr(),
        PrimitiveValue::from(anonymized_value),
    );
    Ok(Some(Cow::Owned(new_elem)))
}

fn process_change_action_hash_date<'a, H: Hasher>(
    elem: &'a InMemElement,
    hasher: &H,
    other_value: String,
) -> Result<Option<Cow<'a, InMemElement>>> {
    if is_empty_element(elem) {
        return Ok(Some(Cow::Borrowed(elem)));
    }

    let value_anonymizer = HashDateAnonymizer::new(hasher, other_value);
    let elem_value_as_string = elem.value().string()?;
    let anonymized_value = value_anonymizer.anonymize(elem_value_as_string)?;

    let new_elem = DataElement::new::<PrimitiveValue>(
        elem.tag(),
        elem.vr(),
        PrimitiveValue::from(anonymized_value),
    );
    Ok(Some(Cow::Owned(new_elem)))
}

fn process_action_empty(elem: &InMemElement) -> Result<Option<Cow<InMemElement>>> {
    let new_elem = DataElement::new::<PrimitiveValue>(elem.tag(), elem.vr(), PrimitiveValue::Empty);
    Ok(Some(Cow::Owned(new_elem)))
}

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
    use crate::config::ConfigBuilder;
    use dicom_core::header::HasLength;
    use dicom_core::{header, VR};
    use dicom_dictionary_std::tags;
    use dicom_object::meta::FileMetaTableBuilder;
    use dicom_object::{FileDicomObject, FileMetaTable, InMemDicomObject};

    fn make_file_meta() -> FileMetaTable {
        FileMetaTableBuilder::new()
            .media_storage_sop_class_uid("1.2.3")
            .media_storage_sop_instance_uid("2.3.4")
            .transfer_syntax("1.2.840.10008.1.2.1") // Explicit VR Little Endian
            .build()
            .unwrap()
    }

    #[test]
    fn test_is_empty_element() {
        let elem = InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::Primitive(PrimitiveValue::Empty),
        );
        assert!(is_empty_element(&elem));
    }

    #[test]
    fn test_process_change_action_replace() {
        let elem = InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::from("0123456789ABCDEF"),
        );
        let processed = process_change_action_replace(&elem, "new_value_123").unwrap();
        assert_eq!(processed.unwrap().value(), &Value::from("new_value_123"));
    }

    #[test]
    fn test_process_change_action_hash() {
        let elem = InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::from("0123456789ABCDEF"),
        );
        let hasher = Blake3Hasher::new();
        let processed = process_change_action_hash(&elem, &hasher, None).unwrap();
        assert_eq!(processed.unwrap().value().length(), header::Length(16));
    }

    #[test]
    fn test_process_change_action_hash_with_length() {
        let elem = InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::from("0123456789ABCDEF"),
        );
        let hasher = Blake3Hasher::new();
        let processed = process_change_action_hash(&elem, &hasher, Some(HashLength(10))).unwrap();
        assert_eq!(processed.unwrap().value().length(), header::Length(10));
    }

    #[test]
    fn test_process_change_action_hash_length_more_than_max_length() {
        let elem = InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::from("0123456789ABCDEF"),
        );
        let hasher = Blake3Hasher::new();
        let processed = process_change_action_hash(&elem, &hasher, Some(HashLength(32))).unwrap();
        assert_eq!(processed.unwrap().value().length(), header::Length(16));
    }

    #[test]
    fn test_process_change_action_hash_empty_input_element() {
        let elem = InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::Primitive(PrimitiveValue::Empty),
        );
        let hasher = Blake3Hasher::new();
        let processed = process_change_action_hash(&elem, &hasher, Some(HashLength(8))).unwrap();
        assert_eq!(processed.unwrap().into_owned(), elem);
    }

    #[test]
    fn test_process_change_action_hash_uid() {
        let elem = InMemElement::new(
            tags::STUDY_INSTANCE_UID,
            VR::UI,
            Value::from("12.34.56.78.9"),
        );
        let hasher = Blake3Hasher::new();
        let uid_root = "".parse().unwrap();
        let processed = process_change_action_hash_uid(&elem, &hasher, &uid_root).unwrap();
        // make sure it's cut off at the max length for VR UI (i.e. 64)
        assert_eq!(
            processed.unwrap().into_owned().value().length(),
            header::Length(64)
        );
    }

    #[test]
    fn test_process_change_action_hash_uid_with_root() {
        let elem = InMemElement::new(
            tags::STUDY_INSTANCE_UID,
            VR::UI,
            Value::from("12.34.56.78.9"),
        );
        let hasher = Blake3Hasher::new();
        let uid_root = "9999".parse().unwrap();
        let processed = process_change_action_hash_uid(&elem, &hasher, &uid_root).unwrap();
        // make sure it's cut off at the max length for VR UI (i.e. 64)
        let processed = processed.unwrap();
        let processed = processed.into_owned();
        assert_eq!(processed.value().length(), header::Length(64));
        let processed_value: String = processed.value().to_str().unwrap().into();
        assert!(processed_value.starts_with("9999."));
    }

    #[test]
    fn test_process_change_action_hash_uid_empty_input_element() {
        let elem = InMemElement::new(
            tags::STUDY_INSTANCE_UID,
            VR::UI,
            Value::Primitive(PrimitiveValue::Empty),
        );
        let hasher = Blake3Hasher::new();
        let uid_root = "".parse().unwrap();
        let processed = process_change_action_hash_uid(&elem, &hasher, &uid_root).unwrap();
        assert_eq!(processed.unwrap().into_owned(), elem);
    }

    #[test]
    fn test_process_action_empty() {
        let elem = InMemElement::new(tags::PATIENT_NAME, VR::PN, Value::from("John Doe"));
        let processed = process_action_empty(&elem).unwrap();
        let processed = processed.unwrap();
        assert_eq!(processed.tag(), tags::PATIENT_NAME);
        assert_eq!(processed.vr(), VR::PN);
        assert_eq!(processed.value(), &Value::Primitive(PrimitiveValue::Empty));
    }

    #[test]
    fn test_process_change_action_hash_date() {
        let other_value = "203087";
        let elem = InMemElement::new(tags::STUDY_DATE, VR::DA, Value::from("20010102"));
        let hasher = Blake3Hasher::new();
        let processed =
            process_change_action_hash_date(&elem, &hasher, other_value.into()).unwrap();
        let processed = processed.unwrap();
        let processed = processed.into_owned();
        assert_eq!(processed.value().length(), header::Length(8));
        assert_eq!(processed.value(), &Value::from("20000921"));
    }

    #[test]
    fn test_process_change_action_hash_date_extended_input_date_format() {
        let other_value = "203087";
        let elem = InMemElement::new(tags::STUDY_DATE, VR::DA, Value::from("2001-01-02"));
        let hasher = Blake3Hasher::new();
        let processed =
            process_change_action_hash_date(&elem, &hasher, other_value.into()).unwrap();
        let processed = processed.unwrap();
        assert_eq!(processed.value().length(), header::Length(10));
        assert_eq!(processed.value(), &Value::from("2000-09-21"));
    }

    #[test]
    fn test_process_change_action_hash_date_empty_input_element() {
        let elem = InMemElement::new(
            tags::STUDY_DATE,
            VR::DA,
            Value::Primitive(PrimitiveValue::Empty),
        );
        let hasher = Blake3Hasher::new();
        let processed = process_change_action_hash_date(&elem, &hasher, "123456".into()).unwrap();
        assert_eq!(processed.unwrap().into_owned(), elem);
    }

    #[test]
    fn test_process_element_hash_length() {
        let meta = make_file_meta();
        let mut obj: FileDicomObject<InMemDicomObject> = FileDicomObject::new_empty_with_meta(meta);

        obj.put(InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::from("0123456789ABCDEF"),
        ));

        let config = ConfigBuilder::new()
            .tag_action(tags::ACCESSION_NUMBER, Action::Hash(None))
            .build();

        let elem = obj.element(tags::ACCESSION_NUMBER).unwrap();
        let processor = DataElementProcessor::new(config);
        let processed = processor.process_element(&obj, elem).unwrap();
        assert_eq!(processed.unwrap().value().length(), header::Length(16));
    }

    #[test]
    fn test_process_element_hash_max_length() {
        let meta = make_file_meta();
        let mut obj: FileDicomObject<InMemDicomObject> = FileDicomObject::new_empty_with_meta(meta);

        obj.put(InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::from("0123456789ABCDEF"),
        ));

        let config = ConfigBuilder::new()
            .tag_action(tags::ACCESSION_NUMBER, Action::Hash(Some(HashLength(32))))
            .build();

        let elem = obj.element(tags::ACCESSION_NUMBER).unwrap();
        let processor = DataElementProcessor::new(config);
        let processed = processor.process_element(&obj, elem).unwrap();
        // new value length should have been cut off at the max length for SH VR, which is 16
        assert_eq!(processed.unwrap().value().length(), header::Length(16));
    }

    #[test]
    fn test_process_element_hash_length_with_value() {
        let meta = make_file_meta();
        let mut obj: FileDicomObject<InMemDicomObject> = FileDicomObject::new_empty_with_meta(meta);

        obj.put(InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::from("0123456789ABCDEF"),
        ));

        let config = ConfigBuilder::new()
            .tag_action(tags::ACCESSION_NUMBER, Action::Hash(Some(HashLength(8))))
            .build();

        let elem = obj.element(tags::ACCESSION_NUMBER).unwrap();
        let processor = DataElementProcessor::new(config);
        let processed = processor.process_element(&obj, elem).unwrap();
        assert_eq!(processed.unwrap().value().length(), header::Length(8));
    }

    #[test]
    fn test_process_element_replace() {
        let meta = make_file_meta();
        let mut obj: FileDicomObject<InMemDicomObject> = FileDicomObject::new_empty_with_meta(meta);

        obj.put(InMemElement::new(
            tags::PATIENT_NAME,
            VR::PN,
            Value::from("John Doe"),
        ));

        let config = ConfigBuilder::new()
            .tag_action(tags::PATIENT_NAME, Action::Replace("Jane Doe".into()))
            .build();

        let elem = obj.element(tags::PATIENT_NAME).unwrap();
        let processor = DataElementProcessor::new(config);
        let processed = processor.process_element(&obj, elem).unwrap();
        assert_eq!(processed.unwrap().value(), &Value::from("Jane Doe"));
    }

    #[test]
    fn test_process_element_keep() {
        let meta = make_file_meta();
        let mut obj: FileDicomObject<InMemDicomObject> = FileDicomObject::new_empty_with_meta(meta);

        obj.put(InMemElement::new(
            tags::PATIENT_NAME,
            VR::PN,
            Value::from("John Doe"),
        ));

        let config = ConfigBuilder::new()
            .tag_action(tags::PATIENT_NAME, Action::Keep)
            .build();

        let elem = obj.element(tags::PATIENT_NAME).unwrap();
        let processor = DataElementProcessor::new(config);
        let processed = processor.process_element(&obj, elem).unwrap();
        assert_eq!(&processed.unwrap().into_owned(), elem);
    }

    #[test]
    fn test_process_element_empty() {
        let meta = make_file_meta();
        let mut obj: FileDicomObject<InMemDicomObject> = FileDicomObject::new_empty_with_meta(meta);

        obj.put(InMemElement::new(
            tags::PATIENT_NAME,
            VR::PN,
            Value::from("John Doe"),
        ));

        let config = ConfigBuilder::new()
            .tag_action(tags::PATIENT_NAME, Action::Empty)
            .build();

        let elem = obj.element(tags::PATIENT_NAME).unwrap();
        let processor = DataElementProcessor::new(config);
        let processed = processor.process_element(&obj, elem).unwrap();
        assert_eq!(
            processed.unwrap().value(),
            &Value::Primitive(PrimitiveValue::Empty)
        );
    }

    #[test]
    fn test_process_element_remove() {
        let meta = make_file_meta();
        let mut obj: FileDicomObject<InMemDicomObject> = FileDicomObject::new_empty_with_meta(meta);

        obj.put(InMemElement::new(
            tags::PATIENT_NAME,
            VR::PN,
            Value::from("John Doe"),
        ));

        let config = ConfigBuilder::new()
            .tag_action(tags::PATIENT_NAME, Action::Remove)
            .build();

        let elem = obj.element(tags::PATIENT_NAME).unwrap();
        let processor = DataElementProcessor::new(config);
        let processed = processor.process_element(&obj, elem).unwrap();
        assert_eq!(processed, None);
    }

    #[test]
    fn test_do_nothing_processor() {
        let meta = make_file_meta();
        let mut obj: FileDicomObject<InMemDicomObject> = FileDicomObject::new_empty_with_meta(meta);

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
