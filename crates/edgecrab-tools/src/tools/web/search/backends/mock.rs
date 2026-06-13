//! Mock backend for chain/fallback unit tests.

use async_trait::async_trait;
use std::sync::{Arc, Mutex};

use crate::tools::web::search::backend::{SearchResult, WebSearchBackend};
use crate::tools::web::search::config::{ExtractOptions, SearchOptions};
use crate::tools::web::search::content_extract::{ExtractHttpError, RawExtractPage};
use crate::tools::web::search::error::SearchError;

#[derive(Debug, Clone)]
pub enum MockMode {
    Success(Vec<SearchResult>),
    RateLimit,
    Timeout,
    Server(u16),
    Network,
    BadRequest(u16),
    Hard,
}

#[derive(Debug, Clone)]
pub enum MockExtractMode {
    Success(RawExtractPage),
    Hard,
}

pub struct MockBackend {
    pub name: &'static str,
    mode: Arc<Mutex<MockMode>>,
    extract_mode: Arc<Mutex<Option<MockExtractMode>>>,
}

impl MockBackend {
    pub fn new(name: &'static str, mode: MockMode) -> Self {
        Self {
            name,
            mode: Arc::new(Mutex::new(mode)),
            extract_mode: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_extract(name: &'static str, mode: MockMode, extract: MockExtractMode) -> Self {
        Self {
            name,
            mode: Arc::new(Mutex::new(mode)),
            extract_mode: Arc::new(Mutex::new(Some(extract))),
        }
    }

    pub fn set_mode(&self, mode: MockMode) {
        *self.mode.lock().expect("mock lock") = mode;
    }
}

#[async_trait]
impl WebSearchBackend for MockBackend {
    fn name(&self) -> &str {
        self.name
    }

    fn is_available(&self) -> bool {
        true
    }

    fn supports_extract(&self) -> bool {
        self.extract_mode.lock().expect("mock lock").is_some()
    }

    async fn search(
        &self,
        _query: &str,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let mode = self.mode.lock().expect("mock lock").clone();
        match mode {
            MockMode::Success(results) => Ok(results
                .into_iter()
                .take(opts.max_results())
                .enumerate()
                .map(|(i, r)| SearchResult::new(i + 1, r.title, r.url, r.snippet, r.source))
                .collect()),
            MockMode::RateLimit => Err(SearchError::rate_limit(self.name, "429 simulated")),
            MockMode::Timeout => Err(SearchError::timeout(self.name, "timeout simulated")),
            MockMode::Server(code) => Err(SearchError::server(self.name, code, "5xx simulated")),
            MockMode::Network => Err(SearchError::network(self.name, "network simulated")),
            MockMode::BadRequest(code) => {
                Err(SearchError::bad_request(self.name, code, "400 simulated"))
            }
            MockMode::Hard => Err(SearchError::hard(self.name, "hard failure")),
        }
    }

    async fn extract(
        &self,
        url: &str,
        _opts: &ExtractOptions,
    ) -> Result<RawExtractPage, ExtractHttpError> {
        let mode = self
            .extract_mode
            .lock()
            .expect("mock lock")
            .clone()
            .ok_or_else(|| {
                ExtractHttpError::hard(format!("{} does not support extract", self.name))
            })?;
        match mode {
            MockExtractMode::Success(mut page) => {
                if page.url.is_empty() {
                    page.url = url.to_string();
                }
                Ok(page)
            }
            MockExtractMode::Hard => Err(ExtractHttpError::hard("mock extract failure")),
        }
    }
}
