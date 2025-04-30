use dicom_core::value::Value;
use dicom_core::PrimitiveValue;
use dicom_object::mem::InMemElement;

pub(crate) fn is_empty_element(elem: &InMemElement) -> bool {
    elem.value() == &Value::Primitive(PrimitiveValue::Empty)
}

pub(crate) fn truncate_to(n: usize, s: &str) -> String {
    s.chars().take(n).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tags;
    use dicom_core::VR;

    #[test]
    fn test_is_empty_element() {
        let elem = InMemElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            Value::Primitive(PrimitiveValue::Empty),
        );
        assert!(is_empty_element(&elem));
    }

    #[test]
    fn test_truncate_to_empty_string() {
        let uid = "";
        let truncated = truncate_to(5, uid);
        assert!(truncated.is_empty());
    }

    #[test]
    fn test_truncate_to_empty_string_to_zero() {
        let uid = "";
        let truncated = truncate_to(0, uid);
        assert!(truncated.is_empty());
    }

    #[test]
    fn test_truncate_to() {
        let uid = "12345";
        let truncated = truncate_to(3, uid);
        assert_eq!(truncated, "123");
    }

    #[test]
    fn test_truncate_to_zero() {
        let uid = "12345";
        let truncated = truncate_to(0, uid);
        assert!(truncated.is_empty());
    }
}
