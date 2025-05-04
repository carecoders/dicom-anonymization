use num_bigint::{BigInt, ParseBigIntError};
use num_traits::Num;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("Invalid input: {}", .0.to_lowercase())]
    InvalidInput(String),
}

impl From<ParseBigIntError> for Error {
    fn from(err: ParseBigIntError) -> Self {
        Error::InvalidInput(format!("{err}"))
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

// signature type for hash functions
pub type HashFn = fn(&str) -> Result<BigInt>;

// blake3 implementation of a hash function
pub fn blake3_hash_fn(input: &str) -> Result<BigInt> {
    let bytes = input.as_bytes();
    let hash = blake3::hash(bytes);
    let hash_as_number = BigInt::from_str_radix(hash.to_hex().as_str(), 16)?;
    Ok(hash_as_number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world() {
        let result = blake3_hash_fn("hello, world!").unwrap();
        assert!(!result.to_string().is_empty());
    }

    #[test]
    fn test_empty_string() {
        let result = blake3_hash_fn("").unwrap();
        assert!(!result.to_string().is_empty());
    }

    #[test]
    fn test_special_characters() {
        let result = blake3_hash_fn("_!@€±§%^!&@*_+{}:?><,.;").unwrap();
        assert!(!result.to_string().is_empty());
    }

    #[test]
    fn test_same_result_for_same_input() {
        let result1 = blake3_hash_fn("abc").unwrap();
        let result2 = blake3_hash_fn("abc").unwrap();
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_different_result_for_different_input() {
        let result1 = blake3_hash_fn("abc").unwrap();
        let result2 = blake3_hash_fn("def").unwrap();
        assert_ne!(result1, result2);
    }
}
