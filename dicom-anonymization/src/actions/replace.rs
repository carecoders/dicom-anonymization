use dicom_core::header::Header;
use dicom_core::value::Value;
use dicom_object::mem::InMemElement;
use dicom_object::DefaultDicomObject;
use std::borrow::Cow;

use crate::actions::errors::ActionError;
use crate::actions::DataElementAction;
use crate::config::Config;

#[derive(Debug, Clone, PartialEq)]
pub struct Replace {
    new_value: String,
}

impl Replace {
    pub fn new(new_value: String) -> Self {
        Self { new_value }
    }
}

impl DataElementAction for Replace {
    fn process<'a>(
        &'a self,
        _config: &Config,
        _obj: &DefaultDicomObject,
        elem: &'a InMemElement,
    ) -> Result<Option<Cow<'a, InMemElement>>, ActionError> {
        let new_elem =
            InMemElement::new(elem.tag(), elem.vr(), Value::from(self.new_value.clone()));
        Ok(Some(Cow::Owned(new_elem)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use dicom_core::value::Value;
    use dicom_core::VR;
    use dicom_object::FileDicomObject;

    use crate::tags;
    use crate::test_utils::make_file_meta;

    #[test]
    fn test_process() {
        let mut obj = FileDicomObject::new_empty_with_meta(make_file_meta());
        let elem = InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::from("0123456789ABCDEF"),
        );
        obj.put(elem.clone());

        let action_struct = Replace::new("new_value_123".into());
        let config = Config::default();

        let processed = action_struct.process(&config, &obj, &elem).unwrap();
        let processed = processed.unwrap();
        let processed = processed.into_owned();
        assert_eq!(processed.value(), &Value::from("new_value_123"));
    }
}
