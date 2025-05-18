use crate::actions::Action;
use dicom_core::{DataDictionary, Tag};
use dicom_dictionary_std::StandardDataDictionary;
use garde::Validate;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct TagActionMap(pub(crate) BTreeMap<Tag, Action>);

pub struct TagActionMapIter<'a> {
    inner: std::collections::btree_map::Iter<'a, Tag, Action>,
}

impl<'a> Iterator for TagActionMapIter<'a> {
    type Item = (&'a Tag, &'a Action);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a> IntoIterator for &'a TagActionMap {
    type Item = (&'a Tag, &'a Action);
    type IntoIter = TagActionMapIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for TagActionMap {
    type Item = (Tag, Action);
    type IntoIter = std::collections::btree_map::IntoIter<Tag, Action>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

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

    pub fn iter(&self) -> TagActionMapIter {
        TagActionMapIter {
            inner: self.0.iter(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
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
            let tag_str = format!("{:04X}{:04X}", tag.group(), tag.element());

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
                    "Tag must be in format 'GGGGEEEE' where G and E are hex digits, got: {}",
                    tag_str
                ))
            })?;

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

#[cfg(test)]
mod tests {
    use super::*;

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
            r#"{"00100010":{"comment":"PatientName","action":"empty"},"00100020":{"comment":"PatientID","action":"remove"}}"#
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
            r#"{"00100010":{"comment":"PatientName","action":"empty"},"00100020":{"comment":"PatientID","action":"remove"}}"#
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
            r#"{"00100010":{"comment":"PatientName","action":"hash"},"00100020":{"comment":"PatientID","action":"remove"},"00200010":{"comment":"StudyID","action":"empty"}}"#
        );
    }

    #[test]
    fn test_error_handling() {
        // Test invalid hex digits
        let json = r#"{"ZZZZ0010":{"action":"empty"}}"#;
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
        assert!(json.contains("\"00100010\":{\"comment\":\"PatientName\",\"action\":\"empty\"}"));

        // For the unknown tag, the comment should be omitted
        assert!(json.contains("\"99999999\":{\"action\":\"remove\"}"));
        assert!(!json.contains("\"99999999\":{\"comment\""));
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

    #[test]
    fn test_iterator_implementation() {
        let mut map = TagActionMap::new();

        // Add some test data
        map.insert(Tag(0x0010, 0x0010), Action::Empty); // PatientName
        map.insert(Tag(0x0010, 0x0020), Action::Remove); // PatientID
        map.insert(Tag(0x0008, 0x0050), Action::HashUID); // AccessionNumber

        // Test the iterator method
        let mut count = 0;
        for (tag, action) in map.iter() {
            count += 1;
            match tag.0 {
                0x0010 => match tag.1 {
                    0x0010 => assert_eq!(*action, Action::Empty),
                    0x0020 => assert_eq!(*action, Action::Remove),
                    _ => panic!("Unexpected element in tag"),
                },
                0x0008 => {
                    assert_eq!(tag.1, 0x0050);
                    assert_eq!(*action, Action::HashUID);
                }
                _ => panic!("Unexpected group in tag"),
            }
        }
        assert_eq!(count, 3);

        // Test the IntoIterator implementation for &TagActionMap
        let vec: Vec<(&Tag, &Action)> = (&map).into_iter().collect();
        assert_eq!(vec.len(), 3);

        // Test the consuming IntoIterator implementation
        let vec: Vec<(Tag, Action)> = map.into_iter().collect();
        assert_eq!(vec.len(), 3);
    }
}
