use dicom_core::header::Header;
use dicom_core::{DataElement, PrimitiveValue};
use dicom_object::mem::InMemElement;
use dicom_object::DefaultDicomObject;
use std::borrow::Cow;

use crate::actions::errors::ActionError;
use crate::actions::utils::{is_empty_element, truncate_to};
use crate::actions::DataElementAction;
use crate::config::uid_root::UidRoot;
use crate::config::Config;
use crate::hasher::HashFn;

const UID_MAX_LENGTH: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub struct HashUID;

impl HashUID {
    fn anonymize(
        &self,
        hash_fn: HashFn,
        uid: &str,
        uid_root: &UidRoot,
    ) -> Result<String, ActionError> {
        let anonymized_uid_as_number = hash_fn(uid)?;
        let anonymized_uid = anonymized_uid_as_number.to_string();
        let extra = if anonymized_uid.starts_with("0") {
            "9"
        } else {
            ""
        };
        let new_uid = format!("{}{}{}", uid_root.as_prefix(), extra, anonymized_uid);
        let result = truncate_to(UID_MAX_LENGTH, &new_uid);

        Ok(result)
    }
}

impl DataElementAction for HashUID {
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
        let uid_root = match config.get_uid_root() {
            Some(uid_root) => uid_root,
            None => &UidRoot("".into()),
        };
        let elem_value = elem.value().string()?;
        let anonymized_value = self.anonymize(hash_fn, elem_value, uid_root)?;

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
    use num_bigint::BigInt;

    use crate::config::builder::ConfigBuilder;
    use crate::hasher::blake3_hash_fn;
    use crate::tags;
    use crate::test_utils::make_file_meta;

    #[test]
    fn test_process() {
        let mut obj = FileDicomObject::new_empty_with_meta(make_file_meta());
        let elem = InMemElement::new(
            tags::STUDY_INSTANCE_UID,
            VR::UI,
            Value::from("12.34.56.78.9"),
        );
        obj.put(elem.clone());

        let action_struct = HashUID;
        let config = Config::default();

        let processed = action_struct.process(&config, &obj, &elem).unwrap();
        // make sure it's cut off at the max length for VR UI (i.e. 64)
        assert_eq!(
            processed.unwrap().into_owned().value().length(),
            header::Length(64)
        );
    }

    #[test]
    fn test_process_with_uid_root() {
        let mut obj = FileDicomObject::new_empty_with_meta(make_file_meta());
        let elem = InMemElement::new(
            tags::STUDY_INSTANCE_UID,
            VR::UI,
            Value::from("12.34.56.78.9"),
        );
        obj.put(elem.clone());

        let action_struct = HashUID;
        let uid_root = "9999".parse().unwrap();
        let config = ConfigBuilder::new().uid_root(uid_root).build();

        let processed = action_struct.process(&config, &obj, &elem).unwrap();
        let processed = processed.unwrap();
        let processed = processed.into_owned();
        assert_eq!(processed.value().length(), header::Length(64));
        let processed_value: String = processed.value().to_str().unwrap().into();
        assert!(processed_value.starts_with("9999."));
    }

    #[test]
    fn test_process_with_empty_element() {
        let mut obj = FileDicomObject::new_empty_with_meta(make_file_meta());
        let elem = InMemElement::new(
            tags::STUDY_INSTANCE_UID,
            VR::UI,
            Value::Primitive(PrimitiveValue::Empty),
        );
        obj.put(elem.clone());

        let action_struct = HashUID;
        let config = Config::default();

        let processed = action_struct.process(&config, &obj, &elem).unwrap();
        assert_eq!(processed.unwrap().into_owned(), elem);
    }

    #[test]
    fn test_anonymize_without_prefix() {
        let uid = "1.2.3.4.5";
        let uid_root = "".parse().unwrap();
        let hash_fn = blake3_hash_fn;
        let action_struct = HashUID;
        let result = action_struct.anonymize(hash_fn, uid, &uid_root);
        let result = result.unwrap();
        assert_eq!(result.len(), 64);
        assert!(!result.contains("."));
    }

    #[test]
    fn test_anonymize_with_prefix() {
        let uid = "1.2.3.4.5";
        let prefix = "2.16.840";
        let uid_root = prefix.parse().unwrap();
        let hash_fn = blake3_hash_fn;
        let action_struct = HashUID;
        let result = action_struct.anonymize(hash_fn, uid, &uid_root);
        let result = result.unwrap();
        assert_eq!(result.len(), 64);
        assert!(result.starts_with("2.16.840."));
    }

    #[test]
    fn test_anonymize_with_empty_prefix() {
        let uid = "1.2.3.4.5";
        let prefix = "";
        let uid_root = prefix.parse().unwrap();
        let hash_fn = blake3_hash_fn;
        let action_struct = HashUID;
        let result = action_struct.anonymize(hash_fn, uid, &uid_root);
        let result = result.unwrap();
        assert_eq!(result.len(), 64);
        assert!(!result.contains("."));
    }

    #[test]
    fn test_anonymize_with_prefix_with_dot() {
        let uid = "1.2.3.4.5";
        let prefix = "2.16.840.";
        let uid_root = prefix.parse().unwrap();
        let hash_fn = blake3_hash_fn;
        let action_struct = HashUID;
        let result = action_struct.anonymize(hash_fn, uid, &uid_root);
        let result = result.unwrap();
        assert_eq!(result.len(), 64);
        assert!(result.starts_with("2.16.840."));
        assert!(!result.starts_with("2.16.840.."));
    }

    #[test]
    fn test_anonymize_with_long_uid() {
        let uid = "1.2.3.4.5.6.7.8.9.10.11.12.13.14.15.16.17.18.19.20.21.22.23.24.25.26.27";
        let prefix = "2.16.840";
        let uid_root = prefix.parse().unwrap();
        let hash_fn = blake3_hash_fn;
        let action_struct = HashUID;
        let result = action_struct.anonymize(hash_fn, uid, &uid_root);
        let result = result.unwrap();
        assert_eq!(result.len(), 64);
        assert!(result.starts_with("2.16.840."));
    }

    #[test]
    fn test_anonymize_with_non_zero_first_digit() {
        let uid = "1.2.3.4.5";
        let prefix = "2.16.840";
        let uid_root = prefix.parse().unwrap();
        let hash_fn =
            |_input: &str| -> Result<BigInt, crate::hasher::Error> { Ok(BigInt::from(123456789)) };
        let action_struct = HashUID;
        let result = action_struct.anonymize(hash_fn, uid, &uid_root);
        assert_eq!(result.unwrap(), "2.16.840.123456789");
    }
}
