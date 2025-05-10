use crate::actions::Action;
use dicom_core::{DataDictionary, Tag};
use dicom_dictionary_std::StandardDataDictionary;
use garde::Validate;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct TagActionMap(pub(crate) BTreeMap<Tag, Action>);

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

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[allow(dead_code)]
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
