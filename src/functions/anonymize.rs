use crate::functions::errors::AnonymizeError;

pub(crate) trait Anonymize {
    fn anonymize(&self, value: &str) -> Result<String, AnonymizeError>;
}
