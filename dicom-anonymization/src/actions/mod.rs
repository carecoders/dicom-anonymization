mod empty;
pub(crate) mod errors;
pub mod hash;
mod hash_date;
mod hash_uid;
mod keep;
mod no_action;
mod remove;
mod replace;
mod utils;

use crate::actions::errors::ActionError;
use crate::actions::hash::HASH_LENGTH_MINIMUM;
use crate::config::Config;
use crate::Tag;
use dicom_object::mem::InMemElement;
use dicom_object::DefaultDicomObject;
use empty::Empty;
use garde::Validate;
use hash::{Hash, HashLength};
use hash_date::HashDate;
use hash_uid::HashUID;
use keep::Keep;
use no_action::NoAction;
use remove::Remove;
use replace::Replace;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;

pub(crate) trait DataElementAction {
    fn process<'a>(
        &'a self,
        config: &Config,
        obj: &DefaultDicomObject,
        elem: &'a InMemElement,
    ) -> Result<Option<Cow<'a, InMemElement>>, ActionError>;
}

#[derive(Debug, Clone)]
pub struct TagString(pub Tag);

impl Serialize for TagString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let tag_str = format!("{}", self.0);
        serializer.serialize_str(&tag_str)
    }
}

impl<'de> Deserialize<'de> for TagString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let tag_str = String::deserialize(deserializer)?;

        let tag: Tag = tag_str.parse().map_err(|_| {
            serde::de::Error::custom(format!(
                "Tag must be in format 'GGGGEEEE' where G and E are hex digits, got: {}",
                tag_str
            ))
        })?;

        Ok(TagString(tag))
    }
}

/// Specifies the action to perform on DICOM data elements during processing.
#[derive(Validate, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum Action {
    /// Clear the value of the data element.
    Empty,

    /// Hash the data element value using an optional custom hash length.
    Hash {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[garde(inner(range(min = HASH_LENGTH_MINIMUM)))]
        length: Option<usize>,
    },

    /// Change a date, using a hash of the PatientID value to determine the offset.
    HashDate,

    /// Generate a new unique identifier (UID) by hashing the original UID.
    HashUID,

    /// Preserve the original data element value without modification.
    Keep,

    /// No action specified.
    None,

    /// Completely remove the data element from the DICOM dataset.
    Remove,

    /// Replace the data element value with the specified string.
    Replace {
        #[garde(skip)]
        value: String,
    },
}

impl Action {
    pub(crate) fn get_action_struct(&self) -> Box<dyn DataElementAction> {
        match self {
            Action::Empty => Box::new(Empty),
            Action::Hash { length } => {
                let hash_length = length.as_ref().map(|length| HashLength(*length));
                Box::new(Hash::new(hash_length))
            }
            Action::HashDate => Box::new(HashDate::default()),
            Action::HashUID => Box::new(HashUID),
            Action::Keep => Box::new(Keep),
            Action::None => Box::new(NoAction),
            Action::Remove => Box::new(Remove),
            Action::Replace { value } => Box::new(Replace::new(value.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Action;
    use serde_json;

    #[test]
    fn test_serialize_empty() {
        let action = Action::Empty;
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, r#"{"action":"empty"}"#);
    }

    #[test]
    fn test_serialize_hash() {
        let action = Action::Hash { length: Some(10) };
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, r#"{"action":"hash","length":10}"#);
    }

    #[test]
    fn test_serialize_hash_date() {
        let action = Action::HashDate;
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, r#"{"action":"hashdate"}"#);
    }

    #[test]
    fn test_serialize_hash_uid() {
        let action = Action::HashUID;
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, r#"{"action":"hashuid"}"#);
    }

    #[test]
    fn test_serialize_keep() {
        let action = Action::Keep;
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, r#"{"action":"keep"}"#);
    }

    #[test]
    fn test_serialize_none() {
        let action = Action::None;
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, r#"{"action":"none"}"#);
    }

    #[test]
    fn test_serialize_remove() {
        let action = Action::Remove;
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, r#"{"action":"remove"}"#);
    }

    #[test]
    fn test_serialize_replace() {
        let action = Action::Replace {
            value: "ANONYMIZED".to_string(),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, r#"{"action":"replace","value":"ANONYMIZED"}"#);
    }

    #[test]
    fn test_deserialize_empty() {
        let json = r#"{"action":"empty"}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        assert_eq!(action, Action::Empty);
    }

    #[test]
    fn test_deserialize_hash() {
        let json = r#"{"action":"hash","length":null}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        assert_eq!(action, Action::Hash { length: None });
    }

    #[test]
    fn test_deserialize_hash_date() {
        let json = r#"{"action":"hashdate"}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        assert_eq!(action, Action::HashDate);
    }

    #[test]
    fn test_deserialize_hash_uid() {
        let json = r#"{"action":"hashuid"}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        assert_eq!(action, Action::HashUID);
    }

    #[test]
    fn test_deserialize_keep() {
        let json = r#"{"action":"keep"}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        assert_eq!(action, Action::Keep);
    }

    #[test]
    fn test_deserialize_none() {
        let json = r#"{"action":"none"}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        assert_eq!(action, Action::None);
    }

    #[test]
    fn test_deserialize_remove() {
        let json = r#"{"action":"remove"}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        assert_eq!(action, Action::Remove);
    }

    #[test]
    fn test_deserialize_replace() {
        let json = r#"{"action":"replace","value":"ANONYMIZED"}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        assert_eq!(
            action,
            Action::Replace {
                value: "ANONYMIZED".to_string()
            }
        );
    }

    #[test]
    fn test_case_handling_on_deserialization() {
        // This test passes - lowercase is expected
        let json = r#"{"action":"empty"}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        assert_eq!(action, Action::Empty);

        // Uppercase will fail without aliases
        let json = r#"{"action":"EMPTY"}"#;
        let result: Result<Action, _> = serde_json::from_str(json);
        assert!(result.is_err());

        // Same for mixed case
        let json = r#"{"action":"Hash"}"#;
        let result: Result<Action, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_roundtrip_all_variants() {
        // Test all variants in one go
        let variants = vec![
            Action::Empty,
            Action::Hash { length: None },
            Action::HashDate,
            Action::HashUID,
            Action::Keep,
            Action::None,
            Action::Remove,
            Action::Replace {
                value: "TEST".to_string(),
            },
        ];

        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: Action = serde_json::from_str(&json).unwrap();
            assert_eq!(
                variant, deserialized,
                "Roundtrip failed for variant: {:?}",
                variant
            );
        }
    }

    #[test]
    fn test_error_handling_missing_action() {
        let json = r#"{"with":"ANONYMIZED"}"#;
        let result: Result<Action, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_handling_invalid_action() {
        let json = r#"{"action":"invalidaction"}"#;
        let result: Result<Action, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_handling_missing_replace_with() {
        let json = r#"{"action":"replace"}"#;
        let result: Result<Action, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_pretty_print() {
        let action = Action::Replace {
            value: "ANONYMIZED".to_string(),
        };
        let json = serde_json::to_string_pretty(&action).unwrap();
        let expected = r#"{
  "action": "replace",
  "value": "ANONYMIZED"
}"#;
        assert_eq!(json, expected);
    }
}
