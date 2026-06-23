//! Structured errors for web search backends and fallback chains.

use edgecrab_types::ToolError;
use std::fmt;

/// Failure modes that drive fallback policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchErrorKind {
    RateLimit,
    Timeout,
    Server(u16),
    Network,
    BadRequest(u16),
    /// Missing API key / endpoint — skip to next backend in chain.
    NotConfigured,
    Hard,
}

/// Error from a single backend or from an exhausted chain.
#[derive(Debug, Clone)]
pub struct SearchError {
    pub kind: SearchErrorKind,
    pub backend: String,
    pub message: String,
}

impl SearchError {
    pub fn rate_limit(backend: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: SearchErrorKind::RateLimit,
            backend: backend.into(),
            message: message.into(),
        }
    }

    pub fn timeout(backend: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: SearchErrorKind::Timeout,
            backend: backend.into(),
            message: message.into(),
        }
    }

    pub fn server(backend: impl Into<String>, code: u16, message: impl Into<String>) -> Self {
        Self {
            kind: SearchErrorKind::Server(code),
            backend: backend.into(),
            message: message.into(),
        }
    }

    pub fn network(backend: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: SearchErrorKind::Network,
            backend: backend.into(),
            message: message.into(),
        }
    }

    pub fn bad_request(backend: impl Into<String>, code: u16, message: impl Into<String>) -> Self {
        Self {
            kind: SearchErrorKind::BadRequest(code),
            backend: backend.into(),
            message: message.into(),
        }
    }

    pub fn hard(backend: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: SearchErrorKind::Hard,
            backend: backend.into(),
            message: message.into(),
        }
    }

    /// Whether the chain should try the next backend.
    pub fn is_fallback_eligible(&self) -> bool {
        matches!(
            self.kind,
            SearchErrorKind::RateLimit
                | SearchErrorKind::Timeout
                | SearchErrorKind::Server(_)
                | SearchErrorKind::Network
                | SearchErrorKind::NotConfigured
        )
    }

    /// Missing credentials for a named backend (uses shared setup message text).
    pub fn not_configured(backend: impl Into<String>) -> Self {
        let backend = backend.into();
        let message = super::backend_settings::not_configured_message(&backend);
        Self {
            kind: SearchErrorKind::NotConfigured,
            backend: backend.clone(),
            message,
        }
    }

    pub fn from_http_status(
        backend: impl Into<String>,
        code: u16,
        message: impl Into<String>,
    ) -> Self {
        let backend = backend.into();
        let message = message.into();
        match code {
            429 => Self::rate_limit(backend, message),
            408 => Self::timeout(backend, message),
            500..=599 => Self::server(backend, code, message),
            400..=499 => Self::bad_request(backend, code, message),
            _ => Self::hard(backend, message),
        }
    }

    pub fn into_tool_error(self) -> ToolError {
        ToolError::ExecutionFailed {
            tool: "web_search".into(),
            message: self.message,
        }
    }
}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.backend, self.message)
    }
}

/// Summary when every backend in the chain failed.
#[derive(Debug, Clone)]
pub struct ChainFailureSummary {
    pub attempts: Vec<(String, String)>,
}

impl ChainFailureSummary {
    pub fn user_message(&self) -> String {
        if self.attempts.is_empty() {
            return "Web search failed. Run `edgecrab setup web` to configure a backend.".into();
        }
        if self.attempts.len() == 1 {
            let (backend, detail) = &self.attempts[0];
            if backend == "ddgs" {
                return format!(
                    "Web search via ddgs failed: {detail} \
                     Set DDGS_PROXY or configure SEARXNG_URL, BRAVE_API_KEY, or TAVILY_API_KEY \
                     in ~/.edgecrab/.env (or run /web setup)."
                );
            }
            return format!("Web search via {backend} failed: {detail}");
        }
        let tried = self
            .attempts
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(" → ");
        let last = self
            .attempts
            .last()
            .map(|(_, m)| m.as_str())
            .unwrap_or("unknown error");
        format!("Web search failed after trying {tried}. Last error: {last}")
    }

    pub fn into_tool_error(self) -> ToolError {
        ToolError::ExecutionFailed {
            tool: "web_search".into(),
            message: self.user_message(),
        }
    }
}
