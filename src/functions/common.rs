pub(crate) fn truncate_to(n: usize, s: &str) -> String {
    s.chars().take(n).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
