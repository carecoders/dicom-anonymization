pub mod builder;
pub(crate) mod tag_action_map;
pub mod uid_root;

use crate::actions::Action;
use crate::hasher::{blake3_hash_fn, HashFn};
use crate::Tag;
use serde::{Deserialize, Serialize};
use tag_action_map::TagActionMap;
use thiserror::Error;
use uid_root::{UidRoot, UidRootError};

const DEIDENTIFIER: &str = "CARECODERS.IO";

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

    #[serde(default, skip_serializing_if = "Option::is_none")]
    uid_root: Option<UidRoot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    remove_private_tags: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    remove_curves: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    remove_overlays: Option<bool>,

    #[serde(
        default = "TagActionMap::default",
        skip_serializing_if = "TagActionMap::is_empty"
    )]
    tag_actions: TagActionMap,
}

impl Config {
    fn new(
        hash_fn: HashFn,
        uid_root: Option<UidRoot>,
        remove_private_tags: Option<bool>,
        remove_curves: Option<bool>,
        remove_overlays: Option<bool>,
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
        Self::new(blake3_hash_fn, None, None, None, None)
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

    pub fn get_uid_root(&self) -> &Option<UidRoot> {
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
        (self.remove_private_tags.unwrap_or(false) && is_private_tag(tag))
            || (self.remove_curves.unwrap_or(false) && is_curve_tag(tag))
            || (self.remove_overlays.unwrap_or(false) && is_overlay_tag(tag))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tags;

    use builder::ConfigBuilder;
    use uid_root::UidRoot;

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
            uid_root: Some(UidRoot("1.2.826.0.1.3680043.10.188".to_string())),
            tag_actions: create_sample_tag_actions(),
            remove_private_tags: Some(true),
            remove_curves: Some(false),
            remove_overlays: Some(true),
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
        assert!(json.contains(r#""00100010""#)); // Patient Name
        assert!(json.contains(r#""action": "empty""#));
        assert!(json.contains(r#""00100020""#)); // Patient ID
        assert!(json.contains(r#""action": "remove""#));
        assert!(json.contains(r#""00080050""#)); // Accession Number
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
        assert_eq!(config.uid_root.unwrap().0, "1.2.826.0.1.3680043.10.188");
        assert_eq!(config.remove_private_tags, Some(true));
        assert_eq!(config.remove_curves, Some(false));
        assert_eq!(config.remove_overlays, Some(true));

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
            uid_root: Some(UidRoot("1.2.826.0.1.3680043.10.188".to_string())),
            tag_actions: create_sample_tag_actions(),
            remove_private_tags: Some(true),
            remove_curves: Some(false),
            remove_overlays: Some(true),
            ..Default::default()
        };

        // Serialize to JSON and back
        let json = serde_json::to_string(&original_config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();

        // Compare UID root
        assert_eq!(
            original_config.uid_root.unwrap().0,
            deserialized.uid_root.unwrap().0
        );

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
            uid_root: Some(UidRoot("1.2.826.0.1.3680043.10.188".to_string())),
            tag_actions: empty_map,
            ..Default::default()
        };

        // Serialize and deserialize
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(
            deserialized.uid_root.unwrap().0,
            "1.2.826.0.1.3680043.10.188"
        );
        assert_eq!(deserialized.remove_private_tags, None);
        assert_eq!(deserialized.remove_curves, None);
        assert_eq!(deserialized.remove_overlays, None);
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

        assert_eq!(config.uid_root.unwrap().0, "1.2.826.0.1.3680043.10.188");
        assert_eq!(config.remove_private_tags, None);
        assert_eq!(config.remove_curves, None);
        assert_eq!(config.remove_overlays, None);
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

        assert_eq!(config.uid_root.unwrap().0, "");
        assert_eq!(config.remove_private_tags, Some(true));
        assert_eq!(config.remove_curves, Some(false));
        assert_eq!(config.remove_overlays, Some(true));
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

        assert_eq!(config.uid_root, None);
        assert_eq!(config.remove_private_tags, Some(true));
        assert_eq!(config.remove_curves, Some(false));
        assert_eq!(config.remove_overlays, Some(true));
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

        assert_eq!(config.uid_root, Some(UidRoot("9999".into())));
        assert_eq!(config.remove_private_tags, None);
        assert_eq!(config.remove_curves, None);
        assert_eq!(config.remove_overlays, None);
        assert_eq!(config.tag_actions.len(), 0);
    }

    #[test]
    fn test_only_empty_tag_actions() {
        let json = r#"{
            "tag_actions": {}
        }"#;

        let result: Result<Config, _> = serde_json::from_str(json);
        let config = result.unwrap();

        assert_eq!(config.uid_root, None);
        assert_eq!(config.remove_private_tags, None);
        assert_eq!(config.remove_curves, None);
        assert_eq!(config.remove_overlays, None);
        assert_eq!(config.tag_actions.len(), 0);
    }

    #[test]
    fn test_empty_json() {
        let json = r#"{}"#;

        let result: Result<Config, _> = serde_json::from_str(json);
        let config = result.unwrap();

        assert_eq!(config.uid_root, None);
        assert_eq!(config.remove_private_tags, None);
        assert_eq!(config.remove_curves, None);
        assert_eq!(config.remove_overlays, None);
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
