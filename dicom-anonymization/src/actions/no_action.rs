use dicom_object::DefaultDicomObject;
use dicom_object::mem::InMemElement;
use std::borrow::Cow;

use crate::actions::ProcessElement;
use crate::actions::errors::ActionError;
use crate::config::Config;

/// Action that performs no operation on DICOM elements.
///
/// This action is functionally equivalent to `Keep` but semantically represents
/// an explicit decision to perform no anonymization action on an element.
#[derive(Debug, Clone, PartialEq)]
pub struct NoAction;

impl ProcessElement for NoAction {
    fn process<'a>(
        &'a self,
        _config: &Config,
        _obj: &DefaultDicomObject,
        elem: &'a InMemElement,
    ) -> Result<Option<Cow<'a, InMemElement>>, ActionError> {
        Ok(Some(Cow::Borrowed(elem)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use dicom_core::VR;
    use dicom_core::value::Value;
    use dicom_object::FileDicomObject;
    use dicom_object::mem::InMemElement;

    use crate::config::Config;
    use crate::tags;
    use crate::test_utils::make_file_meta;

    #[test]
    fn test_process() {
        let obj = FileDicomObject::new_empty_with_meta(make_file_meta());
        let elem = InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::from("0123456789ABCDEF"),
        );

        let result = NoAction.process(&Config::default(), &obj, &elem);
        match result {
            Ok(Some(cow)) => assert_eq!(cow.into_owned(), elem),
            _ => panic!("unexpected result"),
        }
    }
}
