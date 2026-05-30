//! Pluggable web search backend trait and shared result types.

use async_trait::async_trait;
use serde::Serialize;

use super::SearchOptions;
use super::config::ExtractOptions;
use super::content_extract::{ExtractHttpError, RawExtractPage};
use super::error::SearchError;

/// Normalized search hit — all backends map into this shape.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SearchResult {
    pub rank: usize,
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source: String,
}

impl SearchResult {
    pub fn new(
        rank: usize,
        title: impl Into<String>,
        url: impl Into<String>,
        snippet: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            rank,
            title: title.into(),
            url: url.into(),
            snippet: snippet.into(),
            source: source.into(),
        }
    }
}

/// Contract for a web search provider (built-in or plugin-registered).
#[async_trait]
pub trait WebSearchBackend: Send + Sync {
    /// Stable lowercase identifier (`searxng`, `brave`, `ddgs`, …).
    fn name(&self) -> &str;

    /// Human-readable label (Hermes `display_name`; defaults to [`Self::name`]).
    fn display_name(&self) -> &str {
        self.name()
    }

    /// Cheap availability probe — must not perform network I/O.
    fn is_available(&self) -> bool;

    /// Whether this backend can service `web_search` (Hermes `supports_search`).
    fn supports_search(&self) -> bool {
        true
    }

    /// Whether this backend can service `web_extract` via its API (Hermes `supports_extract`).
    fn supports_extract(&self) -> bool {
        false
    }

    /// Whether this backend can service `web_crawl` via its API (Hermes crawl-capable providers).
    fn supports_crawl(&self) -> bool {
        false
    }

    /// Picker metadata for setup wizards (Hermes `get_setup_schema`).
    fn setup_schema(&self) -> super::setup_schema::SetupSchema {
        super::setup_schema::setup_schema_for_backend(self.name())
    }

    /// Execute a search and return normalized results.
    ///
    /// An empty vector is success (not an error).
    async fn search(
        &self,
        query: &str,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchResult>, SearchError>;

    /// Extract content from a single URL (Hermes `WebSearchProvider.extract`).
    ///
    /// Default rejects when [`Self::supports_extract`] is false.
    async fn extract(
        &self,
        url: &str,
        opts: &ExtractOptions,
    ) -> Result<RawExtractPage, ExtractHttpError> {
        let _ = (url, opts);
        Err(ExtractHttpError::hard(format!(
            "{} does not support extract",
            self.name()
        )))
    }
}
