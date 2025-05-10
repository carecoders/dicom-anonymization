pub mod builder;
pub mod profile;

use crate::actions::Action;
use crate::hasher::{blake3_hash_fn, HashFn};
use crate::Tag;
use dicom_core::DataDictionary;
use dicom_dictionary_std::StandardDataDictionary;
use garde::Validate;
use regex::Regex;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::OnceLock;
use thiserror::Error;

static UID_ROOT_REGEX: OnceLock<Regex> = OnceLock::new();

const UID_ROOT_MAX_LENGTH: usize = 32;
const UID_ROOT_DEFAULT_VALUE: &str = "9999";
const DEIDENTIFIER: &str = "CARECODERS";

/// The [`UidRoot`] struct represents a DICOM UID root that can be used as prefix for
/// generating new UIDs during de-identification.
///
/// The [`UidRoot`] must follow DICOM UID format rules:
/// - Start with a digit 1-9
/// - Contain only numbers and dots
///
/// It also must not have more than 32 characters.
///
/// # Example
///
/// ```
/// use dicom_anonymization::config::UidRoot;
///
/// // Create a valid UID root
/// let uid_root = "1.2.840.123".parse::<UidRoot>().unwrap();
///
/// // Invalid UID root (not starting with 1-9)
/// let invalid = "0.1.2".parse::<UidRoot>();
/// assert!(invalid.is_err());
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct UidRoot(String);

#[derive(Error, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[error("{0} is not a valid UID root")]
pub struct UidRootError(String);

#[derive(Error, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum ConfigError {
    #[error("invalid UID root: {0}")]
    InvalidUidRoot(String),

    #[error("invalid hash length: {0}")]
    InvalidHashLength(String),
}

impl From<UidRootError> for ConfigError {
    fn from(err: UidRootError) -> Self {
        ConfigError::InvalidUidRoot(err.0)
    }
}

impl UidRoot {
    pub fn new(uid_root: &str) -> Result<Self, UidRootError> {
        let regex = UID_ROOT_REGEX.get_or_init(|| {
            Regex::new(&format!(
                r"^([1-9][0-9.]{{0,{}}})?$",
                UID_ROOT_MAX_LENGTH - 1
            ))
            .unwrap()
        });

        if !regex.is_match(uid_root) {
            return Err(UidRootError(format!(
                "UID root must be empty or start with 1-9, contain only numbers and dots, and be no longer than {UID_ROOT_MAX_LENGTH} characters"
            )));
        }

        Ok(Self(uid_root.into()))
    }

    /// Returns a string representation of the [`UidRoot`] suitable for use as a UID prefix.
    ///
    /// If the [`UidRoot`] is not empty and does not end with a dot, a dot is appended.
    /// Whitespace is trimmed from both ends in all cases.
    ///
    /// # Returns
    ///
    /// A `String` containing the formatted UID prefix
    pub fn as_prefix(&self) -> String {
        if !self.0.is_empty() && !self.0.ends_with('.') {
            format!("{}.", self.0.trim())
        } else {
            self.0.trim().into()
        }
    }
}

impl Default for UidRoot {
    /// Default implementation for [`UidRoot`] that returns a [`UidRoot`] instance
    /// initialized with an empty string.
    fn default() -> Self {
        Self("".into())
    }
}

impl FromStr for UidRoot {
    type Err = UidRootError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        UidRoot::new(s)
    }
}

impl AsRef<str> for UidRoot {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagActionMap(BTreeMap<Tag, Action>);

impl TagActionMap {
    pub fn new() -> Self {
        TagActionMap(BTreeMap::new())
    }

    pub fn insert(&mut self, tag: Tag, action: Action) -> Option<Action> {
        self.0.insert(tag, action)
    }

    pub fn get(&self, tag: &Tag) -> Option<&Action> {
        self.0.get(tag)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for TagActionMap {
    fn default() -> Self {
        Self::new()
    }
}

// Struct to hold the action and an optional comment
#[derive(Serialize)]
struct TagActionWithComment<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<&'a str>,
    #[serde(flatten)]
    action: &'a Action,
}

// For deserialization, we need an owned version
#[derive(Deserialize)]
struct OwnedTagActionWithComment {
    #[serde(default)]
    #[allow(dead_code)]
    comment: Option<String>,
    #[serde(flatten)]
    action: Action,
}

// Function to get the tag alias from the data dictionary
fn get_tag_alias(tag: &Tag) -> Option<&'static str> {
    let data_dict = StandardDataDictionary;
    let data_entry = data_dict.by_tag(*tag);
    match data_entry {
        Some(entry) => Some(entry.alias),
        _ => None,
    }
}

impl Serialize for TagActionMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;

        for (tag, action) in &self.0 {
            // Try to get the alias for this tag
            let alias = get_tag_alias(tag);

            // Convert tag to string format
            let tag_str = format!("{}", tag);

            // Create the combined structure with an optional comment
            let action_with_desc = TagActionWithComment {
                comment: alias,
                action,
            };

            map.serialize_entry(&tag_str, &action_with_desc)?;
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for TagActionMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Helper type to capture the intermediate representation
        let string_map: BTreeMap<String, OwnedTagActionWithComment> =
            BTreeMap::deserialize(deserializer)?;

        // Convert string map to Tag map
        let mut tag_map = BTreeMap::new();

        for (tag_str, action_with_comment) in string_map {
            // Parse the tag string
            let tag: Tag = tag_str.parse().map_err(|_| {
                serde::de::Error::custom(format!(
                    "Tag must be in format '(XXXX,XXXX)' where X is a hex digit, got: {}",
                    tag_str
                ))
            })?;

            // Make sure the tag string starts and ends with parentheses
            if !tag_str.starts_with('(') || !tag_str.ends_with(')') {
                return Err(serde::de::Error::custom(format!(
                    "Tag must be in format '(XXXX,XXXX)', got: {}",
                    tag_str
                )));
            }

            let action = action_with_comment.action;

            // Make sure the action is valid
            action.validate().map_err(|err| {
                serde::de::Error::custom(format!("Validation error for tag {}: {}", tag_str, err))
            })?;

            // We only keep the action, not the comment
            tag_map.insert(tag, action);
        }

        Ok(TagActionMap(tag_map))
    }
}

pub fn default_hash_fn() -> HashFn {
    blake3_hash_fn
}

/// Configuration for DICOM de-identification.
///
/// This struct contains all the settings that control how DICOM objects will be de-identified, including
/// UID handling, tag-specific actions, and policies for special tag groups.
///
/// # Fields
///
/// * `hash_fn` - The hash function used for all operations requiring hashing
/// * `uid_root` - The [`UidRoot`] to use as prefix when generating new UIDs during de-identification
/// * `remove_private_tags` - Policy determining whether to keep or remove private DICOM tags
/// * `remove_curves` - Policy determining whether to keep or remove curve data (groups `0x5000-0x50FF`)
/// * `remove_overlays` - Policy determining whether to keep or remove overlay data (groups `0x6000-0x60FF`)
/// * `tag_actions` - Mapping of specific DICOM tags to their corresponding de-identification actions
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Config {
    #[serde(skip, default = "default_hash_fn")]
    hash_fn: HashFn,

    #[serde(default)]
    uid_root: UidRoot,

    #[serde(default)]
    remove_private_tags: bool,
    #[serde(default)]
    remove_curves: bool,
    #[serde(default)]
    remove_overlays: bool,

    #[serde(default = "TagActionMap::default")]
    tag_actions: TagActionMap,
}

impl Config {
    fn new(
        hash_fn: HashFn,
        uid_root: UidRoot,
        remove_private_tags: bool,
        remove_curves: bool,
        remove_overlays: bool,
    ) -> Self {
        Self {
            hash_fn,
            uid_root,
            remove_private_tags,
            remove_curves,
            remove_overlays,
            tag_actions: TagActionMap::new(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new(blake3_hash_fn, UidRoot::default(), false, false, false)
    }
}

pub(crate) fn is_private_tag(tag: &Tag) -> bool {
    // tags with odd group numbers are private tags
    tag.group() % 2 != 0
}

pub(crate) fn is_curve_tag(tag: &Tag) -> bool {
    (tag.group() & 0xFF00) == 0x5000
}

pub(crate) fn is_overlay_tag(tag: &Tag) -> bool {
    (tag.group() & 0xFF00) == 0x6000
}

impl Config {
    pub fn get_hash_fn(&self) -> HashFn {
        self.hash_fn
    }

    pub fn get_uid_root(&self) -> &UidRoot {
        &self.uid_root
    }

    /// Returns the appropriate [`Action`] to take for a given DICOM tag.
    ///
    /// This function determines what action should be taken for a specific tag during de-identification
    /// by checking:
    /// 1. If the tag has an explicit action defined in `tag_actions`
    /// 2. Whether the tag should be removed based on the configuration for tag groups (i.e. private tags, curves, overlays)
    ///
    /// # Priority Rules
    /// - If the tag has an explicit action configured of `Action::None` but should be removed based on point 2., returns `Action::Remove`
    /// - If the tag has any other explicit action configured, returns that action
    /// - If the tag has no explicit action configured but should be removed based on point 2., returns `Action::Remove`
    /// - If the tag has no explicit action configured and shouldn't be removed based on point 2., returns `Action::Keep`
    ///
    /// # Arguments
    ///
    /// * `tag` - Reference to the DICOM tag to get the action for
    ///
    /// # Returns
    ///
    /// A reference to the appropriate [`Action`] to take for the given tag
    pub fn get_action(&self, tag: &Tag) -> &Action {
        match self.tag_actions.get(tag) {
            Some(action) if action == &Action::None && self.should_be_removed(tag) => {
                &Action::Remove
            }
            Some(action) => action,
            None if self.should_be_removed(tag) => &Action::Remove,
            None => &Action::Keep,
        }
    }

    fn should_be_removed(&self, tag: &Tag) -> bool {
        self.remove_private_tags && is_private_tag(tag)
            || self.remove_curves && is_curve_tag(tag)
            || self.remove_overlays && is_overlay_tag(tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tags;

    use builder::ConfigBuilder;

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .tag_action(tags::PATIENT_NAME, Action::Empty)
            .build();
        let tag_action = config.get_action(&tags::PATIENT_NAME);
        assert_eq!(tag_action, &Action::Empty);

        // tags without explicit action should be kept by default
        let tag_action = config.get_action(&tags::PATIENT_ID);
        assert_eq!(tag_action, &Action::Keep);
    }

    #[test]
    fn test_uid_root_validation() {
        // Valid cases
        assert!(UidRoot::new("").is_ok());
        assert!(UidRoot::new("1").is_ok());
        assert!(UidRoot::new("1.2.3").is_ok());
        assert!(UidRoot::new("123.456.").is_ok());
        assert!(UidRoot::new(&"1".repeat(32)).is_ok());

        // Invalid cases
        assert!(UidRoot::new("0123").is_err()); // starts with 0
        assert!(UidRoot::new("a.1.2").is_err()); // contains letter
        assert!(UidRoot::new("1.2.3-4").is_err()); // contains invalid character
        assert!(UidRoot::new(&"1".repeat(33)).is_err()); // too long
    }

    #[test]
    fn test_uid_root_from_str() {
        // Valid cases
        let uid_root: Result<UidRoot, _> = "1.2.736.120".parse();
        assert!(uid_root.is_ok());

        let uid_root: Result<UidRoot, _> = "".parse();
        assert!(uid_root.is_ok());

        // Invalid cases
        let uid_root: Result<UidRoot, _> = "0.1.2".parse();
        assert!(uid_root.is_err());

        let uid_root: Result<UidRoot, _> = "invalid".parse();
        assert!(uid_root.is_err());
    }

    #[test]
    fn test_uid_root_as_ref() {
        // Test empty string
        let uid_root = UidRoot::new("").unwrap();
        assert_eq!(uid_root.as_ref(), "");

        // Test normal UID root
        let uid_root = UidRoot::new("1.2.3").unwrap();
        assert_eq!(uid_root.as_ref(), "1.2.3");

        // Test UID root with trailing dot
        let uid_root = UidRoot::new("1.2.3.").unwrap();
        assert_eq!(uid_root.as_ref(), "1.2.3.");

        // Test using as_ref in a function that expects &str
        fn takes_str(_s: &str) {}
        let uid_root = UidRoot::new("1.2.3").unwrap();
        takes_str(uid_root.as_ref());
    }

    #[test]
    fn test_is_private_tag() {
        // private tags
        assert!(is_private_tag(&Tag::from([1, 0])));
        assert!(is_private_tag(&Tag::from([13, 12])));
        assert!(is_private_tag(&Tag::from([33, 33])));

        // non_private tags
        assert!(!is_private_tag(&tags::ACCESSION_NUMBER));
        assert!(!is_private_tag(&tags::PATIENT_ID));
        assert!(!is_private_tag(&tags::PIXEL_DATA));
    }

    #[test]
    fn test_keep_private_tag() {
        let tag = Tag(0x0033, 0x0010);
        let config = ConfigBuilder::new()
            .remove_private_tags(true)
            .tag_action(tag, Action::Keep)
            .build();

        // explicitly kept private tags should be kept
        let tag_action = config.get_action(&tag);
        assert_eq!(tag_action, &Action::Keep);
        // any other private tag should be removed
        assert_eq!(config.get_action(&Tag(0x0033, 0x1010)), &Action::Remove);
        // any other non-private tag should be kept
        assert_eq!(config.get_action(&tags::PATIENT_ID), &Action::Keep);
    }

    #[test]
    fn test_remove_private_tag() {
        let tag = Tag(0x0033, 0x0010);
        let config = ConfigBuilder::new()
            .remove_private_tags(true)
            .tag_action(tag, Action::None)
            .build();
        let tag_action = config.get_action(&tag);
        assert_eq!(tag_action, &Action::Remove);
        assert_eq!(config.get_action(&Tag(0x0033, 0x1010)), &Action::Remove);
        // any other non-private tag should be kept
        assert_eq!(config.get_action(&tags::PATIENT_ID), &Action::Keep);
    }

    #[test]
    fn test_is_curve_tag() {
        // curve tags
        assert!(is_curve_tag(&Tag::from([0x5000, 0])));
        assert!(is_curve_tag(&Tag::from([0x5010, 0x0011])));
        assert!(is_curve_tag(&Tag::from([0x50FF, 0x0100])));

        // non-curve tags
        assert!(!is_curve_tag(&Tag::from([0x5100, 0])));
        assert!(!is_curve_tag(&Tag::from([0x6000, 0])));
    }

    #[test]
    fn test_keep_curve_tag() {
        let tag = Tag(0x5010, 0x0011);
        let config = ConfigBuilder::new()
            .remove_curves(true)
            .tag_action(tag, Action::Keep)
            .build();

        // explicitly kept curve tags should be kept
        let tag_action = config.get_action(&tag);
        assert_eq!(tag_action, &Action::Keep);
        // any other curve tags should be removed
        assert_eq!(config.get_action(&Tag(0x50FF, 0x0100)), &Action::Remove);
        // any other non-curve tag should be kept
        assert_eq!(config.get_action(&tags::PATIENT_ID), &Action::Keep);
    }

    #[test]
    fn test_remove_curve_tag() {
        let tag = Tag(0x5010, 0x0011);
        let config = ConfigBuilder::new()
            .remove_curves(true)
            .tag_action(tag, Action::None)
            .build();
        let tag_action = config.get_action(&tag);
        assert_eq!(tag_action, &Action::Remove);
        assert_eq!(config.get_action(&Tag(0x50FF, 0x0100)), &Action::Remove);
        // any other non-curve tag should be kept
        assert_eq!(config.get_action(&tags::PATIENT_ID), &Action::Keep);
    }

    #[test]
    fn test_is_overlay_tag() {
        // overlay tags
        assert!(is_overlay_tag(&Tag::from([0x6000, 0])));
        assert!(is_overlay_tag(&Tag::from([0x6010, 0x0011])));
        assert!(is_overlay_tag(&Tag::from([0x60FF, 0x0100])));

        // non-overlay tags
        assert!(!is_overlay_tag(&Tag::from([0x6100, 0])));
        assert!(!is_overlay_tag(&Tag::from([0x5000, 0])));
    }

    #[test]
    fn test_keep_overlay_tag() {
        let tag = Tag(0x6010, 0x0011);
        let config = ConfigBuilder::new()
            .remove_overlays(true)
            .tag_action(tag, Action::Keep)
            .build();

        // explicitly kept overlay tags should be kept
        let tag_action = config.get_action(&tag);
        assert_eq!(tag_action, &Action::Keep);
        // any other overlay tags should be removed
        assert_eq!(config.get_action(&Tag(0x60FF, 0x0100)), &Action::Remove);
        // any other non-overlay tag should be kept
        assert_eq!(config.get_action(&tags::PATIENT_ID), &Action::Keep);
    }

    #[test]
    fn test_remove_overlay_tag() {
        let tag = Tag(0x6010, 0x0011);
        let config = ConfigBuilder::new()
            .remove_overlays(true)
            .tag_action(tag, Action::None)
            .build();
        let tag_action = config.get_action(&tag);
        assert_eq!(tag_action, &Action::Remove);
        assert_eq!(config.get_action(&Tag(0x60FF, 0x0100)), &Action::Remove);
        // any other non-overlay tag should be kept
        assert_eq!(config.get_action(&tags::PATIENT_ID), &Action::Keep);
    }

    #[test]
    fn test_tag_action_map() {
        let tag_actions = vec![
            (Tag(0x0010, 0x0010), Action::Empty),
            (Tag(0x0010, 0x0020), Action::Remove),
        ];

        let mut map = TagActionMap::new();
        for tag_action in tag_actions {
            map.insert(tag_action.0, tag_action.1.clone());
        }
        let json = serde_json::to_string(&map).unwrap();

        // Check that the JSON format has tag strings as keys
        assert_eq!(
            json,
            r#"{"(0010,0010)":{"comment":"PatientName","action":"empty"},"(0010,0020)":{"comment":"PatientID","action":"remove"}}"#
        );

        // Test deserialization
        let deserialized: TagActionMap = serde_json::from_str(&json).unwrap();

        // Check tag lookup
        let action1 = deserialized.get(&Tag(0x0010, 0x0010)).unwrap();
        let action2 = deserialized.get(&Tag(0x0010, 0x0020)).unwrap();

        assert_eq!(*action1, Action::Empty);
        assert_eq!(*action2, Action::Remove);

        // Check conversion back to tag actions
        let recovered: Vec<(Tag, Action)> = deserialized
            .0
            .iter()
            .map(|(tag, action)| (*tag, action.clone()))
            .collect();
        assert_eq!(recovered.len(), 2);

        // BTreeMap ordered by Tag, so we can verify the exact order
        assert_eq!(recovered[0].0, Tag(0x0010, 0x0010));
        assert_eq!(recovered[0].1, Action::Empty);
        assert_eq!(recovered[1].0, Tag(0x0010, 0x0020));
        assert_eq!(recovered[1].1, Action::Remove);
    }

    #[test]
    fn test_tag_action_map_insert() {
        let mut map = TagActionMap::new();

        // Insert some tag actions
        map.insert(Tag(0x0010, 0x0010), Action::Empty);
        map.insert(Tag(0x0010, 0x0020), Action::Remove);

        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&Tag(0x0010, 0x0010)), Some(&Action::Empty));

        // Serialize and check format
        let json = serde_json::to_string(&map).unwrap();
        assert_eq!(
            json,
            r#"{"(0010,0010)":{"comment":"PatientName","action":"empty"},"(0010,0020)":{"comment":"PatientID","action":"remove"}}"#
        );
    }

    #[test]
    fn test_tag_ordering() {
        let mut map = TagActionMap::new();

        // Add tags in non-sequential order
        map.insert(Tag(0x0020, 0x0010), Action::Empty); // Group 0020 comes after 0010
        map.insert(Tag(0x0010, 0x0020), Action::Remove); // Element 0020 comes after 0010
        map.insert(Tag(0x0010, 0x0010), Action::Hash { length: None }); // Should be first

        // Convert to tag actions - should be in order
        let actions: Vec<(Tag, Action)> = map
            .0
            .iter()
            .map(|(tag, action)| (*tag, action.clone()))
            .collect();

        // Verify order is by group first, then element
        assert_eq!(actions[0].0, Tag(0x0010, 0x0010));
        assert_eq!(actions[1].0, Tag(0x0010, 0x0020));
        assert_eq!(actions[2].0, Tag(0x0020, 0x0010));

        // Serialize and check the string format
        let json = serde_json::to_string(&map).unwrap();
        assert_eq!(
            json,
            r#"{"(0010,0010)":{"comment":"PatientName","action":"hash"},"(0010,0020)":{"comment":"PatientID","action":"remove"},"(0020,0010)":{"comment":"StudyID","action":"empty"}}"#
        );
    }

    #[test]
    fn test_error_handling() {
        // Test invalid hex digits
        let json = r#"{"(ZZZZ,0010)":{"action":"empty"}}"#;
        let result: Result<TagActionMap, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialization_with_optional_comment() {
        let mut map = TagActionMap::new();

        // Add some tags - one with a known comment, one unknown
        map.insert(Tag(0x0010, 0x0010), Action::Empty); // Known: PatientName
        map.insert(Tag(0x9999, 0x9999), Action::Remove); // Unknown

        // Serialize to JSON
        let json = serde_json::to_string(&map).unwrap();

        // For the known tag, a comment should be present
        assert!(json.contains("\"(0010,0010)\":{\"comment\":\"PatientName\",\"action\":\"empty\"}"));

        // For the unknown tag, the comment should be omitted
        assert!(json.contains("\"(9999,9999)\":{\"action\":\"remove\"}"));
        assert!(!json.contains("\"(9999,9999)\":{\"comment\""));
    }

    #[test]
    fn test_deserialization_with_optional_comment() {
        // Test with and without comment
        let json = r#"{
            "(0010,0010)":{"comment":"PatientName","action":"empty"},
            "(0010,0020)":{"action":"remove"}
        }"#;

        // Deserialize
        let map: TagActionMap = serde_json::from_str(json).unwrap();

        // Both should deserialize correctly
        assert_eq!(map.get(&Tag(0x0010, 0x0010)), Some(&Action::Empty));
        assert_eq!(map.get(&Tag(0x0010, 0x0020)), Some(&Action::Remove));
    }

    #[test]
    fn test_roundtrip_with_optional_comment() {
        let mut original = TagActionMap::new();

        // Add a mix of known and unknown tags
        original.insert(Tag(0x0010, 0x0010), Action::Empty); // Known
        original.insert(Tag(0x0008, 0x0050), Action::HashUID); // Known
        original.insert(Tag(0x9999, 0x9999), Action::Remove); // Unknown

        // Serialize
        let json = serde_json::to_string(&original).unwrap();

        // Known tags should have comments
        assert!(json.contains("\"comment\":\"PatientName\""));
        assert!(json.contains("\"comment\":\"AccessionNumber\""));

        // Unknown tag should not have a comment
        assert!(!json.contains("\"(9999,9999)\":{\"comment\""));

        // Deserialize back
        let deserialized: TagActionMap = serde_json::from_str(&json).unwrap();

        // Verify all actions were preserved
        assert_eq!(deserialized.get(&Tag(0x0010, 0x0010)), Some(&Action::Empty));
        assert_eq!(
            deserialized.get(&Tag(0x0008, 0x0050)),
            Some(&Action::HashUID)
        );
        assert_eq!(
            deserialized.get(&Tag(0x9999, 0x9999)),
            Some(&Action::Remove)
        );
    }

    #[test]
    fn test_malformed_json() {
        // Action field of a wrong type
        let json = r#"{"(0010,0010)":{"comment":"PatientName","action":123}}"#;
        let result: Result<TagActionMap, _> = serde_json::from_str(json);

        // Should fail - action is required and must be valid
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_length_error() {
        // Hash length should be at least 8
        let json = r#"{"(0010,0010)":{"comment":"PatientName","action":"hash","length":5}}"#;
        let result: Result<TagActionMap, _> = serde_json::from_str(json);

        // Should fail - hash length must be valid
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string().to_lowercase();
        assert!(error_message.contains("validation error"));
        assert!(error_message.contains("length"));
    }

    fn create_sample_tag_actions() -> TagActionMap {
        let mut map = TagActionMap::new(); // Assuming you have a constructor
        map.insert(Tag(0x0010, 0x0010), Action::Empty); // Patient Name
        map.insert(Tag(0x0010, 0x0020), Action::Remove); // Patient ID
        map.insert(Tag(0x0008, 0x0050), Action::Hash { length: None }); // Accession Number
        map
    }

    #[test]
    fn test_config_serialization() {
        // Create a sample config
        let config = Config {
            uid_root: UidRoot("1.2.826.0.1.3680043.10.188".to_string()),
            tag_actions: create_sample_tag_actions(),
            remove_private_tags: true,
            remove_curves: false,
            remove_overlays: true,
            ..Default::default()
        };

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&config).unwrap();

        // Basic checks on the JSON string
        assert!(json.contains(r#""uid_root": "1.2.826.0.1.3680043.10.188"#));
        assert!(json.contains(r#""remove_private_tags": true"#));
        assert!(json.contains(r#""remove_curves": false"#));
        assert!(json.contains(r#""remove_overlays": true"#));

        // Check tag actions serialized correctly
        assert!(json.contains(r#""(0010,0010)""#)); // Patient Name
        assert!(json.contains(r#""action": "empty""#));
        assert!(json.contains(r#""(0010,0020)""#)); // Patient ID
        assert!(json.contains(r#""action": "remove""#));
        assert!(json.contains(r#""(0008,0050)""#)); // Accession Number
        assert!(json.contains(r#""action": "hash""#));
    }

    #[test]
    fn test_config_deserialization() {
        // JSON representation of config
        let json = r#"{
            "uid_root": "1.2.826.0.1.3680043.10.188",
            "remove_private_tags": true,
            "remove_curves": false,
            "remove_overlays": true,
            "tag_actions": {
                "(0010,0010)": {"action": "empty"},
                "(0010,0020)": {"action": "remove"},
                "(0008,0050)": {"action": "hash"}
            }
        }"#;

        // Deserialize to Config
        let config: Config = serde_json::from_str(json).unwrap();

        // Check basic fields
        assert_eq!(config.uid_root.0, "1.2.826.0.1.3680043.10.188");
        assert!(config.remove_private_tags);
        assert!(!config.remove_curves);
        assert!(config.remove_overlays);

        // Check tag actions
        let patient_name = config.tag_actions.get(&Tag(0x0010, 0x0010)).unwrap();
        match patient_name {
            Action::Empty => { /* expected */ }
            _ => panic!("Expected Empty action for Patient Name"),
        }

        let patient_id = config.tag_actions.get(&Tag(0x0010, 0x0020)).unwrap();
        match patient_id {
            Action::Remove => { /* expected */ }
            _ => panic!("Expected Remove action for Patient ID"),
        }

        let accession = config.tag_actions.get(&Tag(0x0008, 0x0050)).unwrap();
        match accession {
            Action::Hash { length } => {
                assert_eq!(*length, None);
            }
            _ => panic!("Expected Hash action for Accession Number"),
        }
    }

    #[test]
    fn test_config_roundtrip() {
        // Create original config
        let original_config = Config {
            uid_root: UidRoot("1.2.826.0.1.3680043.10.188".to_string()),
            tag_actions: create_sample_tag_actions(),
            remove_private_tags: true,
            remove_curves: false,
            remove_overlays: true,
            ..Default::default()
        };

        // Serialize to JSON and back
        let json = serde_json::to_string(&original_config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();

        // Compare UID root
        assert_eq!(original_config.uid_root.0, deserialized.uid_root.0);

        // Compare boolean flags
        assert_eq!(
            original_config.remove_private_tags,
            deserialized.remove_private_tags
        );
        assert_eq!(original_config.remove_curves, deserialized.remove_curves);
        assert_eq!(
            original_config.remove_overlays,
            deserialized.remove_overlays
        );

        // Compare tag actions
        let tags_to_check = [
            Tag(0x0010, 0x0010), // Patient Name
            Tag(0x0010, 0x0020), // Patient ID
            Tag(0x0008, 0x0050), // Accession Number
        ];

        for tag in &tags_to_check {
            let original_action = original_config.tag_actions.get(tag);
            let deserialized_action = deserialized.tag_actions.get(tag);

            assert_eq!(
                original_action, deserialized_action,
                "Action for tag ({}) didn't roundtrip correctly",
                tag,
            );
        }
    }

    #[test]
    fn test_empty_tag_actions() {
        // Create a config with empty tag actions
        let empty_map = TagActionMap::new();
        let config = Config {
            uid_root: UidRoot("1.2.826.0.1.3680043.10.188".to_string()),
            tag_actions: empty_map,
            ..Default::default()
        };

        // Serialize and deserialize
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.uid_root.0, "1.2.826.0.1.3680043.10.188");
        assert!(!deserialized.remove_private_tags);
        assert!(!deserialized.remove_curves);
        assert!(!deserialized.remove_overlays);
        assert_eq!(deserialized.tag_actions.len(), 0);
    }

    #[test]
    fn test_partial_config_deserialization() {
        let json = r#"{
            "uid_root": "1.2.826.0.1.3680043.10.188",
            "tag_actions": {
                "(0010,0010)": {"action": "empty"}
            }
        }"#;

        let result: Result<Config, _> = serde_json::from_str(json);
        let config = result.unwrap();

        assert_eq!(config.uid_root.0, "1.2.826.0.1.3680043.10.188");
        assert!(!config.remove_private_tags);
        assert!(!config.remove_curves);
        assert!(!config.remove_overlays);
        assert_eq!(config.tag_actions.len(), 1);
    }

    #[test]
    fn test_empty_uid_root_and_tag_actions() {
        let json = r#"{
            "uid_root": "",
            "remove_private_tags": true,
            "remove_curves": false,
            "remove_overlays": true,
            "tag_actions": {}
        }"#;

        let result: Result<Config, _> = serde_json::from_str(json);
        let config = result.unwrap();

        assert_eq!(config.uid_root.0, "");
        assert!(config.remove_private_tags);
        assert!(!config.remove_curves);
        assert!(config.remove_overlays);
        assert_eq!(config.tag_actions.len(), 0);
    }

    #[test]
    fn test_missing_uid_root() {
        let json = r#"{
            "remove_private_tags": true,
            "remove_curves": false,
            "remove_overlays": true,
            "tag_actions": {}
        }"#;

        let result: Result<Config, _> = serde_json::from_str(json);
        let config = result.unwrap();

        assert_eq!(config.uid_root.0, "");
        assert!(config.remove_private_tags);
        assert!(!config.remove_curves);
        assert!(config.remove_overlays);
        assert_eq!(config.tag_actions.len(), 0);
    }

    #[test]
    fn test_default_remove_fields() {
        let json = r#"{
            "uid_root": "9999",
            "tag_actions": {}
        }"#;

        let result: Result<Config, _> = serde_json::from_str(json);
        let config = result.unwrap();

        assert_eq!(config.uid_root.0, "9999");
        assert!(!config.remove_private_tags);
        assert!(!config.remove_curves);
        assert!(!config.remove_overlays);
        assert_eq!(config.tag_actions.len(), 0);
    }

    #[test]
    fn test_only_empty_tag_actions() {
        let json = r#"{
            "tag_actions": {}
        }"#;

        let result: Result<Config, _> = serde_json::from_str(json);
        let config = result.unwrap();

        assert_eq!(config.uid_root.0, "");
        assert!(!config.remove_private_tags);
        assert!(!config.remove_curves);
        assert!(!config.remove_overlays);
        assert_eq!(config.tag_actions.len(), 0);
    }

    #[test]
    fn test_malformed_config() {
        // Invalid tag format
        let json = r#"{
            "uid_root": "1.2.826.0.1.3680043.10.188",
            "remove_private_tags": true,
            "remove_curves": false,
            "remove_overlays": true,
            "tag_actions": {
                "invalid_tag_format": {"action": "empty"}
            }
        }"#;

        let result: Result<Config, _> = serde_json::from_str(json);
        assert!(result.is_err());

        // Invalid action
        let json = r#"{
            "uid_root": "1.2.826.0.1.3680043.10.188",
            "remove_private_tags": true,
            "remove_curves": false,
            "remove_overlays": true,
            "tag_actions": {
                "(0010,0010)": {"action": "invalid_action"}
            },
        }"#;

        let result: Result<Config, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
