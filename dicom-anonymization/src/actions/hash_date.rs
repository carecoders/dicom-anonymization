use crate::actions::errors::ActionError;
use crate::actions::utils::{is_empty_element, truncate_to};
use crate::actions::DataElementAction;
use crate::config::Config;
use crate::hasher::HashFn;
use crate::tags;
use crate::Tag;
use chrono::{Days, NaiveDate};
use dicom_core::header::Header;
use dicom_core::{DataElement, PrimitiveValue};
use dicom_object::mem::InMemElement;
use dicom_object::DefaultDicomObject;
use std::borrow::Cow;
use std::num::ParseIntError;

// support hyphens as well, just in case that format is used as input, even though it's not
// compliant with the DICOM standard
const DATE_SUPPORTED_FORMATS: [&str; 2] = ["%Y%m%d", "%Y-%m-%d"];

fn parse_date(value: &str) -> Result<(NaiveDate, &str, &str), ActionError> {
    DATE_SUPPORTED_FORMATS
        .iter()
        .find_map(|&format| {
            let result = NaiveDate::parse_and_remainder(value, format).ok();
            match result {
                Some((date, remainder)) => Some((date, remainder, format)),
                _ => None,
            }
        })
        .ok_or_else(|| ActionError::InvalidInput(format!("unable to parse date from {}", value)))
}

#[derive(Debug, Clone, PartialEq)]
pub struct HashDate {
    other_tag: Tag,
}

impl HashDate {
    pub fn new(other_tag: Tag) -> Self {
        Self { other_tag }
    }

    fn anonymize(
        &self,
        hash_fn: HashFn,
        value: &str,
        other_value: &str,
    ) -> Result<String, ActionError> {
        let (date, remainder, format) = parse_date(value)?;
        let hash_number = hash_fn(other_value)?;
        let inc_str = truncate_to(4, &hash_number.to_string());

        // Parsing hash string into u64 should always be possible because it only contains decimal
        // numbers, but let's do proper error handling nonetheless.
        let inc_parsed: u64 = inc_str
            .parse()
            .map_err(<ParseIntError as Into<ActionError>>::into)?;

        let inc = inc_parsed % (10 * 365);
        let inc = if inc == 0 { 1 } else { inc };
        let new_date = date - Days::new(inc);
        let result = new_date.format(format).to_string() + remainder;
        Ok(result)
    }
}

impl Default for HashDate {
    fn default() -> Self {
        Self::new(tags::PATIENT_ID)
    }
}

impl DataElementAction for HashDate {
    fn process<'a>(
        &'a self,
        config: &Config,
        obj: &DefaultDicomObject,
        elem: &'a InMemElement,
    ) -> Result<Option<Cow<'a, InMemElement>>, ActionError> {
        if is_empty_element(elem) {
            return Ok(Some(Cow::Borrowed(elem)));
        }

        let other_elem = match obj.element(self.other_tag) {
            Ok(elem) => elem,
            Err(_) => {
                return Err(ActionError::InvalidHashDateTag(format!(
                    "could not change tag {} because the other tag {} is not available",
                    elem.tag(),
                    self.other_tag
                )));
            }
        };

        let other_value = match other_elem.value().string() {
            Ok(value) => value,
            Err(_) => {
                return Err(ActionError::InvalidHashDateTag(format!(
                    "could not change tag {} because the other tag {} does not have a valid value",
                    elem.tag(),
                    self.other_tag
                )));
            }
        };

        let hash_fn = config.get_hash_fn();
        let elem_value = elem.value().string()?;
        let anonymized_value = self.anonymize(hash_fn, elem_value, other_value)?;

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
    use crate::test_utils::make_file_meta;

    #[test]
    fn test_process() {
        let mut obj = FileDicomObject::new_empty_with_meta(make_file_meta());
        let patient_id_elem = InMemElement::new(tags::PATIENT_ID, VR::LO, Value::from("203087"));
        obj.put(patient_id_elem);
        let elem = InMemElement::new(tags::STUDY_DATE, VR::DA, Value::from("20010102"));
        obj.put(elem.clone());

        let action_struct = HashDate::default();
        let config = Config::default();
        let processed = action_struct.process(&config, &obj, &elem).unwrap();
        let processed = processed.unwrap();
        let processed = processed.into_owned();
        assert_eq!(processed.value().length(), header::Length(8));
        assert_eq!(processed.value(), &Value::from("20000921"));
    }

    #[test]
    fn test_process_with_hyphened_date() {
        let mut obj = FileDicomObject::new_empty_with_meta(make_file_meta());
        let patient_id_elem = InMemElement::new(tags::PATIENT_ID, VR::LO, Value::from("203087"));
        obj.put(patient_id_elem);
        let elem = InMemElement::new(tags::STUDY_DATE, VR::DA, Value::from("2001-01-02"));
        obj.put(elem.clone());

        let action_struct = HashDate::default();
        let config = Config::default();
        let processed = action_struct.process(&config, &obj, &elem).unwrap();
        let processed = processed.unwrap();
        let processed = processed.into_owned();
        assert_eq!(processed.value().length(), header::Length(10));
        assert_eq!(processed.value(), &Value::from("2000-09-21"));
    }

    #[test]
    fn test_process_with_empty_date_element() {
        let mut obj = FileDicomObject::new_empty_with_meta(make_file_meta());
        let patient_id_elem = InMemElement::new(tags::PATIENT_ID, VR::LO, Value::from("203087"));
        obj.put(patient_id_elem);
        let elem = InMemElement::new(
            tags::STUDY_DATE,
            VR::DA,
            Value::Primitive(PrimitiveValue::Empty),
        );
        obj.put(elem.clone());

        let action_struct = HashDate::default();
        let config = Config::default();
        let processed = action_struct.process(&config, &obj, &elem).unwrap();
        assert_eq!(processed.unwrap().into_owned(), elem);
    }

    #[test]
    fn test_anonymize_with_date_as_input() {
        let hash_fn = blake3_hash_fn;
        let action_struct = HashDate::default();
        let result = action_struct.anonymize(hash_fn, "20010102", "203087");
        let result = result.unwrap();
        assert_eq!(result.len(), 8);
        assert_eq!(result, "20000921");
    }

    #[test]
    fn test_anonymize_with_date_time_as_input() {
        let hash_fn = blake3_hash_fn;
        let action_struct = HashDate::default();
        let result = action_struct.anonymize(hash_fn, "20010102131110", "203087");
        let result = result.unwrap();
        assert_eq!(result, "20000921131110");
    }

    #[test]
    fn test_anonymize_dates_with_same_seed() {
        let seed = "203087";
        let hash_fn = blake3_hash_fn;
        let action_struct = HashDate::default();

        let result = action_struct.anonymize(hash_fn, "20010102", seed);
        let result = result.unwrap();
        assert_eq!(result.len(), 8);
        assert_eq!(result, "20000921");

        let result = action_struct.anonymize(hash_fn, "20000102", seed);
        let result = result.unwrap();
        assert_eq!(result.len(), 8);
        assert_eq!(result, "19990921");
    }

    #[test]
    fn test_parse_date_with_first_date_format() {
        let result = parse_date("20010102");
        assert!(result.is_ok());
        let (date, remainder, format) = result.unwrap();
        assert_eq!(remainder, "");
        assert_eq!(format, "%Y%m%d");
        assert_eq!(date.format("%Y-%m-%d").to_string(), "2001-01-02");
    }

    #[test]
    fn test_parse_date_with_second_date_format() {
        let result = parse_date("2001-01-02");
        assert!(result.is_ok());
        let (date, remainder, format) = result.unwrap();
        assert_eq!(remainder, "");
        assert_eq!(format, "%Y-%m-%d");
        assert_eq!(date.format("%Y%m%d").to_string(), "20010102");
    }

    #[test]
    fn test_parse_date_from_date_time() {
        let result = parse_date("20010102141545");
        assert!(result.is_ok());
        let (date, remainder, format) = result.unwrap();
        assert_eq!(remainder, "141545");
        assert_eq!(format, "%Y%m%d");
        assert_eq!(date.format("%Y%m%d").to_string(), "20010102");
    }

    #[test]
    fn test_parse_date_with_unsupported_date_format() {
        let result = parse_date("2001/01/02");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_string_starting_with_zero() {
        let result: u64 = "0123".parse().unwrap();
        assert_eq!(result, 123);
    }
}
