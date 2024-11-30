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

use dicom_core::Tag;
use dicom_object::mem::InMemElement;
use dicom_object::DefaultDicomObject;
use empty::Empty;
use hash::{Hash, HashLength};
use hash_date::HashDate;
use hash_uid::HashUID;
use keep::Keep;
use no_action::NoAction;
use remove::Remove;
use replace::Replace;
use std::borrow::Cow;

use crate::config::Config;
use crate::processor;

pub trait DataElementAction {
    fn process<'a>(
        &'a self,
        config: &Config,
        obj: &DefaultDicomObject,
        elem: &'a InMemElement,
    ) -> processor::Result<Option<Cow<'a, InMemElement>>>;
}

/// Specifies the action to perform on DICOM data elements during processing.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Clear the value of the data element.
    Empty,

    /// Hash the data element value using an optional custom hash length.
    Hash(Option<HashLength>),

    /// Change a date, using a hash of the given tag value to determine the offset.
    HashDate(Tag),

    /// Generate a new unique identifier (UID) by hashing the original UID.
    HashUID,

    /// Preserve the original data element value without modification.
    Keep,

    /// No action specified.
    None,

    /// Completely remove the data element from the DICOM dataset.
    Remove,

    /// Replace the data element value with the specified string.
    Replace(String),
}

impl Action {
    pub fn get_action_struct(&self) -> Box<dyn DataElementAction> {
        match self {
            Action::Empty => Box::new(Empty),
            Action::Hash(length) => Box::new(Hash::new(*length)),
            Action::HashDate(other_tag) => Box::new(HashDate::new(*other_tag)),
            Action::HashUID => Box::new(HashUID),
            Action::Keep => Box::new(Keep),
            Action::None => Box::new(NoAction),
            Action::Remove => Box::new(Remove),
            Action::Replace(new_value) => Box::new(Replace::new(new_value.clone())),
        }
    }
}
