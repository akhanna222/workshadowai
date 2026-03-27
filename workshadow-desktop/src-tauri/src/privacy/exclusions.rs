/// Checks whether an app or URL should be excluded from capture.
pub struct ExclusionFilter {
    excluded_apps: Vec<String>,
    excluded_url_patterns: Vec<String>,
}

impl ExclusionFilter {
    pub fn new(excluded_apps: Vec<String>, excluded_url_patterns: Vec<String>) -> Self {
        Self {
            excluded_apps,
            excluded_url_patterns,
        }
    }

    /// Returns true if the given app should NOT be captured.
    pub fn is_app_excluded(&self, app_name: &str) -> bool {
        let app_lower = app_name.to_lowercase();
        self.excluded_apps
            .iter()
            .any(|excl| app_lower.contains(&excl.to_lowercase()))
    }

    /// Returns true if the given URL matches an exclusion pattern.
    pub fn is_url_excluded(&self, url: &str) -> bool {
        let url_lower = url.to_lowercase();
        self.excluded_url_patterns.iter().any(|pattern| {
            let pattern_lower = pattern.to_lowercase();
            // Simple glob matching: *pattern* style
            let stripped = pattern_lower.trim_matches('*');
            url_lower.contains(stripped)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_filter() -> ExclusionFilter {
        ExclusionFilter::new(
            vec!["1Password".to_string(), "Bitwarden".to_string()],
            vec!["*bank*".to_string(), "*paypal*".to_string()],
        )
    }

    #[test]
    fn test_app_exclusion() {
        let filter = test_filter();
        assert!(filter.is_app_excluded("1Password 8"));
        assert!(filter.is_app_excluded("bitwarden")); // case-insensitive
        assert!(!filter.is_app_excluded("Visual Studio Code"));
    }

    #[test]
    fn test_url_exclusion() {
        let filter = test_filter();
        assert!(filter.is_url_excluded("https://mybank.com/login"));
        assert!(filter.is_url_excluded("https://www.paypal.com/checkout"));
        assert!(!filter.is_url_excluded("https://github.com"));
    }
}
