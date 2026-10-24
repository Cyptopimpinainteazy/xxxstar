//! Intentionally-defective fixtures that prove external testing tools detect
//! defects. This crate holds no X3 logic and is excluded from the root
//! workspace; it exists so tools can be validated on a known-good / known-bad
//! pair before being trusted on real X3 code.

/// The 14-byte magic that the fuzz fixture treats as a defect.
pub const FORBIDDEN_TAG: &[u8] = b"X3CRASHFIXTURE";

/// True when `data` contains the forbidden tag.
pub fn contains_forbidden_tag(data: &[u8]) -> bool {
    !FORBIDDEN_TAG.is_empty()
        && data
            .windows(FORBIDDEN_TAG.len())
            .any(|w| w == FORBIDDEN_TAG)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_the_tag() {
        assert!(contains_forbidden_tag(b"prefix-X3CRASHFIXTURE-suffix"));
    }

    #[test]
    fn ignores_clean_input() {
        assert!(!contains_forbidden_tag(b"a well formed record"));
        assert!(!contains_forbidden_tag(b""));
    }
}
