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

/// SHA-256 implementation of a hash function.
///
/// The `sha2` crate has no hex encoder, so the digest bytes are hex-encoded
/// manually before being parsed as a BigInt.
pub fn sha256_hash_fn(input: &str) -> Result<BigInt> {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(input.as_bytes());
    let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
    let hash_as_number = BigInt::from_str_radix(hex.as_str(), 16)?;
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

    #[test]
    fn test_sha256_known_vector() {
        // sha256("1.2.3.4.5") as a decimal number — the canonical golden vector
        // (shared with the C#/Python implementations in the consuming project).
        let result = sha256_hash_fn("1.2.3.4.5").unwrap().to_string();
        assert_eq!(
            result,
            "83554103981997929853173016752638312087386577304557461336823424116065032446690"
        );
    }

    #[test]
    fn test_sha256_deterministic() {
        assert_eq!(sha256_hash_fn("abc").unwrap(), sha256_hash_fn("abc").unwrap());
        assert_ne!(sha256_hash_fn("abc").unwrap(), sha256_hash_fn("def").unwrap());
    }
}
