//! Shared authentication utilities.

/// Constant-time string comparison to prevent timing attacks.
///
/// Both inputs are compared byte-by-byte; the result is only
/// determined after all bytes have been examined, preventing an
/// attacker from learning the expected key one character at a time.
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_strings() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(constant_time_eq("", ""));
        assert!(constant_time_eq(
            "tt-0123456789abcdef",
            "tt-0123456789abcdef"
        ));
    }

    #[test]
    fn different_strings() {
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("abc", "ab"));
        assert!(!constant_time_eq("", "a"));
    }
}
