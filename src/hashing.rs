use num_bigint::{BigInt, ParseBigIntError};
use num_traits::Num;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub(crate) enum Error {
    #[error("Invalid input: {}", .0.to_lowercase())]
    InvalidInput(String),
}

impl From<ParseBigIntError> for Error {
    fn from(err: ParseBigIntError) -> Self {
        Error::InvalidInput(format!("{err}"))
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub(crate) trait Hasher {
    fn hash(&self, input: &str) -> Result<String>;
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Blake3Hasher;

impl Blake3Hasher {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl Hasher for Blake3Hasher {
    fn hash(&self, input: &str) -> Result<String> {
        let bytes = input.as_bytes();
        let hash = blake3::hash(bytes);
        let hash_as_number = BigInt::from_str_radix(hash.to_hex().as_str(), 16)?;
        Ok(hash_as_number.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world() {
        let hasher = Blake3Hasher::new();
        let result = hasher.hash("hello, world!").unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_empty_string() {
        let hasher = Blake3Hasher::new();
        let result = hasher.hash("").unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_special_characters() {
        let hasher = Blake3Hasher::new();
        let result = hasher.hash("_!@€±§%^!&@*_+{}:?><,.;").unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_same_result_for_same_input() {
        let hasher = Blake3Hasher::new();
        let result1 = hasher.hash("abc").unwrap();
        let result2 = hasher.hash("abc").unwrap();
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_different_result_for_different_input() {
        let hasher = Blake3Hasher::new();
        let result1 = hasher.hash("abc").unwrap();
        let result2 = hasher.hash("def").unwrap();
        assert_ne!(result1, result2);
    }
}
