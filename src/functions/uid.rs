use super::anonymize::Anonymize;
use super::common::truncate_to;
use super::errors::AnonymizeError;
use crate::config::UidRoot;
use crate::hashing::Hasher;

const UID_MAX_LENGTH: usize = 64;

pub(crate) struct UidAnonymizer<'a, H>
where
    H: Hasher,
{
    hasher: &'a H,
    uid_root: &'a UidRoot,
}

impl<'a, H> UidAnonymizer<'a, H>
where
    H: Hasher,
{
    pub(crate) fn new(hasher: &'a H, uid_root: &'a UidRoot) -> Self {
        Self { hasher, uid_root }
    }
}

impl<H> Anonymize for UidAnonymizer<'_, H>
where
    H: Hasher,
{
    fn anonymize(&self, uid: &str) -> Result<String, AnonymizeError> {
        let anonymized_uid = self.hasher.hash(uid)?;
        let extra = if anonymized_uid.starts_with("0") {
            "9"
        } else {
            ""
        };
        let new_uid = format!("{}{}{}", self.uid_root.as_prefix(), extra, anonymized_uid);
        let result = truncate_to(UID_MAX_LENGTH, &new_uid);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashing::Blake3Hasher;
    use crate::hashing::Error as HashingError;

    #[test]
    fn test_hash_uid_no_prefix() {
        let uid = "1.2.3.4.5";
        let hasher = Blake3Hasher::new();
        let uid_root = "".parse().unwrap();
        let anonymizer = UidAnonymizer::new(&hasher, &uid_root);
        let result = anonymizer.anonymize(uid);
        let result = result.unwrap();
        assert_eq!(result.len(), 64);
        assert!(!result.contains("."));
    }

    #[test]
    fn test_hash_uid_with_prefix() {
        let uid = "1.2.3.4.5";
        let prefix = "2.16.840";
        let uid_root = prefix.parse().unwrap();
        let hasher = Blake3Hasher::new();
        let anonymizer = UidAnonymizer::new(&hasher, &uid_root);
        let result = anonymizer.anonymize(uid);
        let result = result.unwrap();
        assert_eq!(result.len(), 64);
        assert!(result.starts_with("2.16.840."));
    }

    #[test]
    fn test_hash_uid_with_prefix_without_dots() {
        let uid = "1.2.3.4.5";
        let prefix = "9999";
        let uid_root = prefix.parse().unwrap();
        let hasher = Blake3Hasher::new();
        let anonymizer = UidAnonymizer::new(&hasher, &uid_root);
        let result = anonymizer.anonymize(uid);
        let result = result.unwrap();
        assert_eq!(result.len(), 64);
        assert!(result.starts_with("9999."));
    }

    #[test]
    fn test_hash_uid_with_empty_prefix() {
        let uid = "1.2.3.4.5";
        let prefix = "";
        let uid_root = prefix.parse().unwrap();
        let hasher = Blake3Hasher::new();
        let anonymizer = UidAnonymizer::new(&hasher, &uid_root);
        let result = anonymizer.anonymize(uid);
        let result = result.unwrap();
        assert_eq!(result.len(), 64);
    }

    #[test]
    fn test_hash_uid_with_prefix_with_dot() {
        let uid = "1.2.3.4.5";
        let prefix = "2.16.840.";
        let uid_root = prefix.parse().unwrap();
        let hasher = Blake3Hasher::new();
        let anonymizer = UidAnonymizer::new(&hasher, &uid_root);
        let result = anonymizer.anonymize(uid);
        let result = result.unwrap();
        assert_eq!(result.len(), 64);
        assert!(result.starts_with("2.16.840."));
    }

    #[test]
    fn test_hash_uid_long_result() {
        let uid = "1.2.3.4.5.6.7.8.9.10.11.12.13.14.15.16.17.18.19.20.21.22.23.24.25.26.27";
        let prefix = "2.16.840";
        let uid_root = prefix.parse().unwrap();
        let hasher = Blake3Hasher::new();
        let anonymizer = UidAnonymizer::new(&hasher, &uid_root);
        let result = anonymizer.anonymize(uid);
        let result = result.unwrap();
        assert_eq!(result.len(), 64);
        assert!(result.starts_with("2.16.840."));
    }

    #[test]
    fn test_hash_uid_first_digit_zero() {
        let uid = "1.2.3.4.5";
        let prefix = "2.16.840";
        let uid_root = prefix.parse().unwrap();

        struct FakeHasher;
        impl Hasher for FakeHasher {
            fn hash(&self, _input: &str) -> Result<String, HashingError> {
                Ok("0123456789".to_owned())
            }
        }

        let hasher = FakeHasher {};
        let anonymizer = UidAnonymizer::new(&hasher, &uid_root);
        let result = anonymizer.anonymize(uid);
        assert_eq!(result.unwrap(), "2.16.840.90123456789");
    }

    #[test]
    fn test_hash_uid_first_digit_non_zero() {
        let uid = "1.2.3.4.5";
        let prefix = "2.16.840";
        let uid_root = prefix.parse().unwrap();

        struct FakeHasher;
        impl Hasher for FakeHasher {
            fn hash(&self, _input: &str) -> Result<String, HashingError> {
                Ok("123456789".to_owned())
            }
        }

        let hasher = FakeHasher {};
        let anonymizer = UidAnonymizer::new(&hasher, &uid_root);
        let result = anonymizer.anonymize(uid);
        assert_eq!(result.unwrap(), "2.16.840.123456789");
    }
}
