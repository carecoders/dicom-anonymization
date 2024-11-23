use super::anonymize::Anonymize;
use super::common::truncate_to;
use super::errors::AnonymizeError;
use crate::hashing::Hasher;
use chrono::{Days, NaiveDate};

// support hyphens as well, just in case that format is used as input, even though it's not
// compliant with the DICOM standard
const DATE_SUPPORTED_FORMATS: [&str; 2] = ["%Y%m%d", "%Y-%m-%d"];

pub(crate) struct HashDateAnonymizer<'a, H>
where
    H: Hasher,
{
    hasher: &'a H,
    other: String,
}

impl<'a, H> HashDateAnonymizer<'a, H>
where
    H: Hasher,
{
    pub(crate) fn new(hasher: &'a H, other: String) -> Self {
        Self { hasher, other }
    }

    fn parse_date(value: &str) -> Result<(NaiveDate, &str, &str), AnonymizeError> {
        DATE_SUPPORTED_FORMATS
            .iter()
            .find_map(|&format| {
                let result = NaiveDate::parse_and_remainder(value, format).ok();
                match result {
                    Some((date, remainder)) => Some((date, remainder, format)),
                    _ => None,
                }
            })
            .ok_or_else(|| {
                AnonymizeError::InvalidInput(format!("unable to parse date from {}", value))
            })
    }
}

impl<H> Anonymize for HashDateAnonymizer<'_, H>
where
    H: Hasher,
{
    fn anonymize(&self, value: &str) -> Result<String, AnonymizeError> {
        let (date, remainder, format) = HashDateAnonymizer::<H>::parse_date(value)?;
        let hash_string = self.hasher.hash(&self.other)?;
        let inc_str = truncate_to(4, &hash_string);

        // Parsing hash string into u64 should always be possible because it only contains decimal
        // numbers
        let inc_parsed: u64 = inc_str
            .parse()
            .expect("Failed to parse u64 from hash string");

        let inc = inc_parsed % (10 * 365);
        let inc = if inc == 0 { 1 } else { inc };
        let new_date = date - Days::new(inc);
        let result = new_date.format(format).to_string() + remainder;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashing::Blake3Hasher;

    #[test]
    fn test_first_date_format() {
        let result = HashDateAnonymizer::<Blake3Hasher>::parse_date("20010102");
        assert!(result.is_ok());
        let (date, remainder, format) = result.unwrap();
        assert_eq!(remainder, "");
        assert_eq!(format, "%Y%m%d");
        assert_eq!(date.format("%Y-%m-%d").to_string(), "2001-01-02");
    }

    #[test]
    fn test_second_date_format() {
        let result = HashDateAnonymizer::<Blake3Hasher>::parse_date("2001-01-02");
        assert!(result.is_ok());
        let (date, remainder, format) = result.unwrap();
        assert_eq!(remainder, "");
        assert_eq!(format, "%Y-%m-%d");
        assert_eq!(date.format("%Y%m%d").to_string(), "20010102");
    }

    #[test]
    fn test_date_time() {
        let result = HashDateAnonymizer::<Blake3Hasher>::parse_date("20010102141545");
        assert!(result.is_ok());
        let (date, remainder, format) = result.unwrap();
        assert_eq!(remainder, "141545");
        assert_eq!(format, "%Y%m%d");
        assert_eq!(date.format("%Y%m%d").to_string(), "20010102");
    }

    #[test]
    fn test_unsupported_date_format() {
        let result = HashDateAnonymizer::<Blake3Hasher>::parse_date("2001/01/02");
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_date() {
        let patient_id = String::from("203087");
        let hasher = Blake3Hasher::new();
        let anonymizer = HashDateAnonymizer::new(&hasher, patient_id);
        let result = anonymizer.anonymize("20010102");
        let result = result.unwrap();
        assert_eq!(result.len(), 8);
        assert_eq!(result, "20000921");
    }

    #[test]
    fn test_hash_date_time() {
        let patient_id = String::from("203087");
        let hasher = Blake3Hasher::new();
        let anonymizer = HashDateAnonymizer::new(&hasher, patient_id);
        let result = anonymizer.anonymize("20010102131110");
        let result = result.unwrap();
        assert_eq!(result, "20000921131110");
    }

    #[test]
    fn test_hash_dates_with_same_seed() {
        let seed = String::from("203087");
        let hasher = Blake3Hasher::new();
        let anonymizer = HashDateAnonymizer::new(&hasher, seed);

        let result = anonymizer.anonymize("20010102");
        let result = result.unwrap();
        assert_eq!(result.len(), 8);
        assert_eq!(result, "20000921");

        let result = anonymizer.anonymize("20000102");
        let result = result.unwrap();
        assert_eq!(result.len(), 8);
        assert_eq!(result, "19990921");
    }

    #[test]
    fn test_parse_string_starting_with_zero() {
        let result: u64 = "0123".parse().unwrap();
        assert_eq!(result, 123);
    }
}
