use super::anonymize::Anonymize;
use super::common::truncate_to;
use super::errors::AnonymizeError;
use crate::actions::HashLength;
use crate::hashing::Hasher;

pub(crate) struct HashAnonymizer<'a, H>
where
    H: Hasher,
{
    hasher: &'a H,
    length: Option<HashLength>,
}

impl<'a, H> HashAnonymizer<'a, H>
where
    H: Hasher,
{
    pub(crate) fn new(hasher: &'a H, length: Option<HashLength>) -> Self {
        Self { hasher, length }
    }
}

impl<H> Anonymize for HashAnonymizer<'_, H>
where
    H: Hasher,
{
    fn anonymize(&self, value: &str) -> Result<String, AnonymizeError> {
        let anonymized_value = self.hasher.hash(value)?;

        let result = match self.length {
            Some(length) => truncate_to(length.0, &anonymized_value),
            None => anonymized_value,
        };
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashing::Blake3Hasher;

    #[test]
    fn test_hash_no_length() {
        let value = "203087";
        let hasher = Blake3Hasher::new();
        let hash_length = None;
        let anonymizer = HashAnonymizer::new(&hasher, hash_length);
        let result = anonymizer.anonymize(value);
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_hash_with_length() {
        let value = "203087";
        let hasher = Blake3Hasher::new();
        let hash_length = Some(HashLength(10));
        let anonymizer = HashAnonymizer::new(&hasher, hash_length);
        let result = anonymizer.anonymize(value);
        assert_eq!(result.unwrap().len(), 10);
    }
}
