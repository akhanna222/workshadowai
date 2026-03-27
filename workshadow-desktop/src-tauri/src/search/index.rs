use super::{SearchFilters, SearchResult};
use std::path::Path;
use std::sync::Mutex;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, IndexReader, IndexWriter, TantivyDocument};

/// Full-text search index backed by Tantivy.
pub struct SearchIndex {
    index: Index,
    reader: IndexReader,
    writer: Mutex<IndexWriter>,
    // Field handles
    field_frame_id: Field,
    field_timestamp: Field,
    field_ocr_text: Field,
    field_window_title: Field,
    field_app_id: Field,
    field_browser_url: Field,
    uncommitted_count: Mutex<usize>,
}

const COMMIT_BATCH_SIZE: usize = 100;

impl SearchIndex {
    pub fn open(index_dir: &Path) -> tantivy::Result<Self> {
        let mut schema_builder = Schema::builder();

        let field_frame_id = schema_builder.add_i64_field("frame_id", STORED | INDEXED);
        let field_timestamp = schema_builder.add_i64_field("timestamp_ms", STORED | INDEXED);
        let field_ocr_text = schema_builder.add_text_field("ocr_text", TEXT | STORED);
        let field_window_title = schema_builder.add_text_field("window_title", TEXT | STORED);
        let field_app_id = schema_builder.add_text_field("app_id", STRING | STORED);
        let field_browser_url = schema_builder.add_text_field("browser_url", TEXT | STORED);

        let schema = schema_builder.build();

        std::fs::create_dir_all(index_dir).ok();
        let index = Index::create_in_dir(index_dir, schema.clone())
            .or_else(|_| Index::open_in_dir(index_dir))?;

        let writer = index.writer(50_000_000)?; // 50MB buffer
        let reader = index.reader()?;

        Ok(Self {
            index,
            reader,
            writer: Mutex::new(writer),
            field_frame_id,
            field_timestamp,
            field_ocr_text,
            field_window_title,
            field_app_id,
            field_browser_url,
            uncommitted_count: Mutex::new(0),
        })
    }

    /// Index a frame's OCR text and metadata.
    /// Auto-commits after every COMMIT_BATCH_SIZE documents.
    pub fn add_frame(
        &self,
        frame_id: i64,
        timestamp_ms: u64,
        ocr_text: &str,
        window_title: &str,
        app_id: &str,
        browser_url: Option<&str>,
    ) -> tantivy::Result<()> {
        let writer = self.writer.lock().unwrap();

        let mut doc = TantivyDocument::new();
        doc.add_i64(self.field_frame_id, frame_id);
        doc.add_i64(self.field_timestamp, timestamp_ms as i64);
        doc.add_text(self.field_ocr_text, ocr_text);
        doc.add_text(self.field_window_title, window_title);
        doc.add_text(self.field_app_id, app_id);
        if let Some(url) = browser_url {
            doc.add_text(self.field_browser_url, url);
        }

        writer.add_document(doc)?;
        drop(writer);

        // Auto-commit periodically
        let mut count = self.uncommitted_count.lock().unwrap();
        *count += 1;
        if *count >= COMMIT_BATCH_SIZE {
            *count = 0;
            drop(count);
            self.commit()?;
        }

        Ok(())
    }

    /// Commit pending index writes and reload the reader.
    pub fn commit(&self) -> tantivy::Result<()> {
        let mut writer = self.writer.lock().unwrap();
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Search the index using a text query with optional filters.
    pub fn search(
        &self,
        query_str: &str,
        filters: &SearchFilters,
        max_results: usize,
    ) -> tantivy::Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.field_ocr_text,
                self.field_window_title,
                self.field_browser_url,
            ],
        );

        let query = query_parser.parse_query(query_str)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(max_results))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;

            let frame_id = doc
                .get_first(self.field_frame_id)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let timestamp_ms = doc
                .get_first(self.field_timestamp)
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as u64;
            let matched_text = doc
                .get_first(self.field_ocr_text)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let window_title = doc
                .get_first(self.field_window_title)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let app_id = doc
                .get_first(self.field_app_id)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Apply post-query filters
            if let Some(from) = filters.date_from {
                if timestamp_ms < from {
                    continue;
                }
            }
            if let Some(to) = filters.date_to {
                if timestamp_ms > to {
                    continue;
                }
            }
            if let Some(ref ids) = filters.app_ids {
                if !ids.contains(&app_id) {
                    continue;
                }
            }

            results.push(SearchResult {
                frame_id,
                timestamp_ms,
                matched_text,
                window_title,
                app_id,
                relevance_score: score,
            });
        }

        Ok(results)
    }

    /// Get the number of documents in the index.
    pub fn num_docs(&self) -> u64 {
        let searcher = self.reader.searcher();
        searcher.num_docs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_index() -> (tempfile::TempDir, SearchIndex) {
        let dir = tempfile::tempdir().unwrap();
        let index = SearchIndex::open(dir.path()).unwrap();
        (dir, index)
    }

    #[test]
    fn test_add_and_search() {
        let (_dir, index) = setup_index();

        index
            .add_frame(
                1,
                1000,
                "rust programming language systems",
                "Rust Book - Firefox",
                "firefox",
                Some("doc.rust-lang.org"),
            )
            .unwrap();

        index
            .add_frame(
                2,
                2000,
                "python data science machine learning",
                "Jupyter Notebook - Chrome",
                "chrome",
                Some("localhost:8888"),
            )
            .unwrap();

        index.commit().unwrap();

        let results = index
            .search("rust programming", &SearchFilters::default(), 10)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].frame_id, 1);
        assert!(results[0].relevance_score > 0.0);
    }

    #[test]
    fn test_search_with_date_filter() {
        let (_dir, index) = setup_index();

        index.add_frame(1, 1000, "hello world", "Window 1", "app1", None).unwrap();
        index.add_frame(2, 5000, "hello world", "Window 2", "app2", None).unwrap();
        index.commit().unwrap();

        let filters = SearchFilters {
            date_from: Some(3000),
            ..Default::default()
        };
        let results = index.search("hello", &filters, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].frame_id, 2);
    }

    #[test]
    fn test_search_with_app_filter() {
        let (_dir, index) = setup_index();

        index.add_frame(1, 1000, "coding stuff", "Editor", "vscode", None).unwrap();
        index.add_frame(2, 2000, "coding stuff", "Browser", "chrome", None).unwrap();
        index.commit().unwrap();

        let filters = SearchFilters {
            app_ids: Some(vec!["vscode".to_string()]),
            ..Default::default()
        };
        let results = index.search("coding", &filters, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].app_id, "vscode");
    }

    #[test]
    fn test_search_window_title() {
        let (_dir, index) = setup_index();

        index.add_frame(1, 1000, "", "Important Document.pdf - Preview", "preview", None).unwrap();
        index.commit().unwrap();

        let results = index.search("Important Document", &SearchFilters::default(), 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_num_docs() {
        let (_dir, index) = setup_index();
        assert_eq!(index.num_docs(), 0);

        for i in 0..5 {
            index.add_frame(i, i as u64 * 1000, "text", "win", "app", None).unwrap();
        }
        index.commit().unwrap();
        assert_eq!(index.num_docs(), 5);
    }
}
