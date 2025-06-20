use dicom_core::header::Header;
use dicom_core::{DataElement, PrimitiveValue};
use dicom_object::DefaultDicomObject;
use dicom_object::mem::InMemElement;
use std::borrow::Cow;

use crate::actions::ProcessElement;
use crate::actions::errors::ActionError;
use crate::config::Config;

/// Action that empties DICOM element values while preserving the element structure.
///
/// This action replaces the value of a DICOM element with an empty primitive value,
/// effectively removing the data content while keeping the element tag and VR intact.
#[derive(Debug, Clone, PartialEq)]
pub struct Empty;

impl ProcessElement for Empty {
    fn process<'a>(
        &'a self,
        _config: &Config,
        _obj: &DefaultDicomObject,
        elem: &'a InMemElement,
    ) -> Result<Option<Cow<'a, InMemElement>>, ActionError> {
        let new_elem =
            DataElement::new::<PrimitiveValue>(elem.tag(), elem.vr(), PrimitiveValue::Empty);
        Ok(Some(Cow::Owned(new_elem)))
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

        let expected = InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::Primitive(PrimitiveValue::Empty),
        );

        let result = Empty.process(&Config::default(), &obj, &elem);
        match result {
            Ok(Some(cow)) => assert_eq!(cow.into_owned(), expected),
            _ => panic!("unexpected result"),
        }
    }
}
