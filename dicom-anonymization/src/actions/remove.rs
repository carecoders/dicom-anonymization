use dicom_object::mem::InMemElement;
use dicom_object::DefaultDicomObject;
use std::borrow::Cow;

use crate::actions::errors::ActionError;
use crate::actions::ProcessElement;
use crate::config::Config;

/// Action that completely removes DICOM elements from the dataset.
///
/// This action eliminates the element entirely, returning `None` to indicate
/// that the element should not be present in the anonymized output.
#[derive(Debug, Clone, PartialEq)]
pub struct Remove;

impl ProcessElement for Remove {
    fn process<'a>(
        &'a self,
        _config: &Config,
        _obj: &DefaultDicomObject,
        _elem: &'a InMemElement,
    ) -> Result<Option<Cow<'a, InMemElement>>, ActionError> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use dicom_core::value::Value;
    use dicom_core::VR;
    use dicom_object::mem::InMemElement;
    use dicom_object::FileDicomObject;

    use crate::config::Config;
    use crate::tags;
    use crate::test_utils::make_file_meta;

    #[test]
    fn test_process() {
        let mut obj = FileDicomObject::new_empty_with_meta(make_file_meta());
        let elem = InMemElement::new(tags::PATIENT_NAME, VR::PN, Value::from("John Doe"));
        obj.put(elem.clone());

        let action_struct = Remove;
        let config = Config::default();

        let processed = action_struct.process(&config, &obj, &elem).unwrap();
        assert_eq!(processed, None);
    }
}
