pub mod index;

use serde::{Deserialize, Serialize};

/// Search configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub index_dir: String,
    pub max_results: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            index_dir: "~/.workshadow/index".to_string(),
            max_results: 50,
        }
    }
}

/// A search result returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub frame_id: i64,
    pub timestamp_ms: u64,
    pub matched_text: String,
    pub window_title: String,
    pub app_id: String,
    pub relevance_score: f32,
}

/// Filters that can be applied to search queries.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchFilters {
    pub date_from: Option<u64>,
    pub date_to: Option<u64>,
    pub app_ids: Option<Vec<String>>,
    pub url_domains: Option<Vec<String>>,
}
