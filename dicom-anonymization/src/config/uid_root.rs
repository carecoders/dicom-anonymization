use regex::Regex;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::OnceLock;
use thiserror::Error;

static UID_ROOT_REGEX: OnceLock<Regex> = OnceLock::new();
const UID_ROOT_MAX_LENGTH: usize = 32;
pub const UID_ROOT_DEFAULT_VALUE: &str = "9999";

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
/// use dicom_anonymization::config::uid_root::UidRoot;
///
/// // Create a valid UID root
/// let uid_root = "1.2.840.123".parse::<UidRoot>().unwrap();
///
/// // Invalid UID root (not starting with 1-9)
/// let invalid = "0.1.2".parse::<UidRoot>();
/// assert!(invalid.is_err());
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct UidRoot(pub String);

#[derive(Error, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[error("{0} is not a valid UID root")]
pub struct UidRootError(pub String);

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
