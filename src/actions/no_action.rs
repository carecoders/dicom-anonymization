use dicom_object::mem::InMemElement;
use dicom_object::DefaultDicomObject;
use std::borrow::Cow;

use crate::actions::DataElementAction;
use crate::config::Config;
use crate::processor::Error as ProcessorError;

#[derive(Debug, Clone, PartialEq)]
pub struct NoAction;

impl DataElementAction for NoAction {
    fn process<'a>(
        &'a self,
        _config: &Config,
        _obj: &DefaultDicomObject,
        elem: &'a InMemElement,
    ) -> Result<Option<Cow<'a, InMemElement>>, ProcessorError> {
        Ok(Some(Cow::Borrowed(elem)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use dicom_core::value::Value;
    use dicom_core::VR;
    use dicom_dictionary_std::tags;
    use dicom_object::mem::InMemElement;
    use dicom_object::FileDicomObject;

    use crate::config::Config;
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
