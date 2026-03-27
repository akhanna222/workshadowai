use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PiiType {
    Email,
    Phone,
    Ssn,
    CreditCard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiDetection {
    pub pii_type: PiiType,
    pub matched_text: String,
    pub start: usize,
    pub end: usize,
}

static EMAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap());

static PHONE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:\+?1[-.\s]?)?(?:\(?\d{3}\)?[-.\s]?)?\d{3}[-.\s]?\d{4}\b").unwrap());

static SSN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap());

static CREDIT_CARD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:\d{4}[-\s]?){3}\d{4}\b").unwrap());

/// Scan text for PII patterns. Returns detected PII items (flagged, not redacted).
pub fn detect_pii(text: &str) -> Vec<PiiDetection> {
    let mut results = Vec::new();

    for m in EMAIL_RE.find_iter(text) {
        results.push(PiiDetection {
            pii_type: PiiType::Email,
            matched_text: m.as_str().to_string(),
            start: m.start(),
            end: m.end(),
        });
    }

    for m in PHONE_RE.find_iter(text) {
        results.push(PiiDetection {
            pii_type: PiiType::Phone,
            matched_text: m.as_str().to_string(),
            start: m.start(),
            end: m.end(),
        });
    }

    for m in SSN_RE.find_iter(text) {
        results.push(PiiDetection {
            pii_type: PiiType::Ssn,
            matched_text: m.as_str().to_string(),
            start: m.start(),
            end: m.end(),
        });
    }

    for m in CREDIT_CARD_RE.find_iter(text) {
        results.push(PiiDetection {
            pii_type: PiiType::CreditCard,
            matched_text: m.as_str().to_string(),
            start: m.start(),
            end: m.end(),
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_email() {
        let results = detect_pii("Contact us at hello@example.com for info");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].pii_type, PiiType::Email);
        assert_eq!(results[0].matched_text, "hello@example.com");
    }

    #[test]
    fn test_detect_ssn() {
        let results = detect_pii("SSN: 123-45-6789");
        assert!(results.iter().any(|r| r.pii_type == PiiType::Ssn));
    }

    #[test]
    fn test_detect_credit_card() {
        let results = detect_pii("Card: 4111-1111-1111-1111");
        assert!(results.iter().any(|r| r.pii_type == PiiType::CreditCard));
    }

    #[test]
    fn test_no_pii() {
        let results = detect_pii("This is a normal sentence with no sensitive data.");
        assert!(results.is_empty());
    }
}
