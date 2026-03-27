use std::collections::HashSet;

/// Deduplicates OCR text between consecutive frames using Jaccard similarity.
pub struct TextDeduplicator {
    threshold: f32,
    previous_tokens: HashSet<String>,
}

impl TextDeduplicator {
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,
            previous_tokens: HashSet::new(),
        }
    }

    /// Returns true if the new text is too similar to the previous frame (should skip).
    pub fn is_duplicate(&mut self, text: &str) -> bool {
        let current_tokens: HashSet<String> = text
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();

        if current_tokens.is_empty() && self.previous_tokens.is_empty() {
            return true;
        }

        if current_tokens.is_empty() || self.previous_tokens.is_empty() {
            self.previous_tokens = current_tokens;
            return false;
        }

        let intersection = current_tokens.intersection(&self.previous_tokens).count();
        let union = current_tokens.union(&self.previous_tokens).count();
        let similarity = intersection as f32 / union as f32;

        let is_dup = similarity >= self.threshold;
        self.previous_tokens = current_tokens;
        is_dup
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_text_is_duplicate() {
        let mut dedup = TextDeduplicator::new(0.9);
        assert!(!dedup.is_duplicate("hello world foo bar"));
        assert!(dedup.is_duplicate("hello world foo bar"));
    }

    #[test]
    fn test_different_text_is_not_duplicate() {
        let mut dedup = TextDeduplicator::new(0.9);
        assert!(!dedup.is_duplicate("hello world"));
        assert!(!dedup.is_duplicate("completely different text here"));
    }

    #[test]
    fn test_slightly_different_text() {
        let mut dedup = TextDeduplicator::new(0.9);
        assert!(!dedup.is_duplicate("the quick brown fox jumps over the lazy dog"));
        // Change one word out of 9 → Jaccard ~ 0.8, below 0.9 threshold
        assert!(!dedup.is_duplicate("the quick brown cat jumps over the lazy dog"));
    }

    #[test]
    fn test_empty_text() {
        let mut dedup = TextDeduplicator::new(0.9);
        assert!(dedup.is_duplicate(""));
        assert!(dedup.is_duplicate(""));
    }
}
