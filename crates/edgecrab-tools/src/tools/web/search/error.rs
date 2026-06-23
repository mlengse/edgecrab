//! Web search specific errors.

use std::borrow::Cow;

use edgecrab_types::ToolError;

/// Core error type returned by search backends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchError {
    /// The search backend that generated the error.
    pub backend: Cow<'static, str>,
    /// The category of error.
    pub kind: SearchErrorKind,
    /// A human-readable description of the error.
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchErrorKind {
    /// The backend is properly configured, but the remote server returned an HTTP error
    /// (e.g. 500 Internal Server Error, 502 Bad Gateway).
    /// Indicates the service is currently unavailable or broken.
    ServerError(u16),

    /// The remote server rejected the request due to rate limiting or quote exhaustion
    /// (e.g. 429 Too Many Requests, or a parsed 403 Forbidden on Bing/DDG).
    RateLimit,

    /// The client could not connect, TLS failed, or the connection timed out.
    NetworkError,

    /// The client timed out waiting for a response.
    Timeout,

    /// The request failed due to missing or invalid credentials (e.g. 401 Unauthorized, 403 Forbidden).
    AuthError,

    /// The user provided invalid input (e.g. 400 Bad Request), or the backend was misconfigured.
    HardError,
}

impl SearchError {
    pub fn server(backend: impl Into<Cow<'static, str>>, status: u16, message: impl Into<String>) -> Self {
        Self {
            backend: backend.into(),
            kind: SearchErrorKind::ServerError(status),
            message: message.into(),
        }
    }

    pub fn rate_limit(backend: impl Into<Cow<'static, str>>, message: impl Into<String>) -> Self {
        Self {
            backend: backend.into(),
            kind: SearchErrorKind::RateLimit,
            message: message.into(),
        }
    }

    pub fn network(backend: impl Into<Cow<'static, str>>, message: impl Into<String>) -> Self {
        Self {
            backend: backend.into(),
            kind: SearchErrorKind::NetworkError,
            message: message.into(),
        }
    }

    pub fn timeout(backend: impl Into<Cow<'static, str>>, message: impl Into<String>) -> Self {
        Self {
            backend: backend.into(),
            kind: SearchErrorKind::Timeout,
            message: message.into(),
        }
    }

    pub fn auth(backend: impl Into<Cow<'static, str>>, message: impl Into<String>) -> Self {
        Self {
            backend: backend.into(),
            kind: SearchErrorKind::AuthError,
            message: message.into(),
        }
    }

    pub fn hard(backend: impl Into<Cow<'static, str>>, message: impl Into<String>) -> Self {
        Self {
            backend: backend.into(),
            kind: SearchErrorKind::HardError,
            message: message.into(),
        }
    }

    /// Whether this error should skip the current backend and try the next one in the chain.
    pub fn should_fallback(&self) -> bool {
        match self.kind {
            SearchErrorKind::ServerError(_)
            | SearchErrorKind::RateLimit
            | SearchErrorKind::NetworkError
            | SearchErrorKind::Timeout => true,
            SearchErrorKind::AuthError | SearchErrorKind::HardError => false,
        }
    }
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} error: {}", self.backend, self.message)
    }
}

impl std::error::Error for SearchError {}

impl From<SearchError> for ToolError {
    fn from(err: SearchError) -> Self {
        ToolError::ExecutionFailed {
            tool: "web_search".into(),
            message: err.to_string(),
        }
    }
}

/// A summary of failures across all backends attempted in a fallback chain.
#[derive(Debug, Clone, Default)]
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
            return format!("Web search via {backend} failed: {detail}");
        }
        let tried = self
            .attempts
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(" -> ");
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
