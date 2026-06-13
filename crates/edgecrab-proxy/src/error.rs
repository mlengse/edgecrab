//! OpenAI-shaped error responses for the proxy.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("model not found: {0}")]
    ModelNotFound(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("not implemented: {0}")]
    NotImplemented(String),
    #[error("rate limited")]
    RateLimited,
    #[error("service unavailable")]
    ServiceUnavailable,
    #[error("upstream auth failed: {0}")]
    UpstreamAuth(String),
    #[error("path not allowed: {path}; allowed: {allowed:?}")]
    PathNotAllowed { path: String, allowed: Vec<String> },
    #[error("upstream unreachable: {0}")]
    UpstreamUnreachable(String),
    #[error("upstream request timed out")]
    UpstreamTimeout,
    #[error("upstream error: {0}")]
    Upstream(String),
    #[error("provider error: {0}")]
    Provider(edgequake_llm::LlmError),
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, Serialize)]
struct OpenAiErrorBody {
    error: OpenAiErrorDetail,
}

#[derive(Debug, Serialize)]
struct OpenAiErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
    code: Option<String>,
}

impl ProxyError {
    fn status(&self) -> StatusCode {
        match self {
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::ModelNotFound(_) => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::UpstreamAuth(_) => StatusCode::UNAUTHORIZED,
            Self::PathNotAllowed { .. } => StatusCode::NOT_FOUND,
            Self::UpstreamUnreachable(_) => StatusCode::BAD_GATEWAY,
            Self::UpstreamTimeout => StatusCode::GATEWAY_TIMEOUT,
            Self::Provider(err) => match err {
                edgequake_llm::LlmError::ModelNotFound(_) => StatusCode::NOT_FOUND,
                edgequake_llm::LlmError::AuthError(_) => StatusCode::UNAUTHORIZED,
                edgequake_llm::LlmError::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
                edgequake_llm::LlmError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
                edgequake_llm::LlmError::ApiError(m)
                | edgequake_llm::LlmError::ProviderError(m)
                    if is_overloaded(m) =>
                {
                    StatusCode::SERVICE_UNAVAILABLE
                }
                edgequake_llm::LlmError::Timeout => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            Self::Upstream(_) | Self::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn openai_detail(&self) -> OpenAiErrorDetail {
        match self {
            Self::Unauthorized(msg) => OpenAiErrorDetail {
                message: msg.clone(),
                error_type: "invalid_request_error".into(),
                code: Some("invalid_api_key".into()),
            },
            Self::ModelNotFound(msg) => OpenAiErrorDetail {
                message: msg.clone(),
                error_type: "invalid_request_error".into(),
                code: Some("model_not_found".into()),
            },
            Self::BadRequest(msg) => OpenAiErrorDetail {
                message: msg.clone(),
                error_type: "invalid_request_error".into(),
                code: None,
            },
            Self::NotImplemented(msg) => OpenAiErrorDetail {
                message: msg.clone(),
                error_type: "server_error".into(),
                code: Some("not_implemented".into()),
            },
            Self::RateLimited => OpenAiErrorDetail {
                message: "Rate limit exceeded".into(),
                error_type: "rate_limit_error".into(),
                code: Some("rate_limit_exceeded".into()),
            },
            Self::ServiceUnavailable => OpenAiErrorDetail {
                message: "Service temporarily unavailable".into(),
                error_type: "server_error".into(),
                code: Some("service_unavailable".into()),
            },
            Self::UpstreamAuth(msg) => OpenAiErrorDetail {
                message: msg.clone(),
                error_type: "invalid_request_error".into(),
                code: Some("upstream_auth_failed".into()),
            },
            Self::PathNotAllowed { path, allowed } => OpenAiErrorDetail {
                message: format!("path {path} not forwarded; allowed: {allowed:?}"),
                error_type: "invalid_request_error".into(),
                code: Some("path_not_allowed".into()),
            },
            Self::UpstreamUnreachable(msg) => OpenAiErrorDetail {
                message: msg.clone(),
                error_type: "server_error".into(),
                code: Some("upstream_unreachable".into()),
            },
            Self::UpstreamTimeout => OpenAiErrorDetail {
                message: "upstream request timed out".into(),
                error_type: "server_error".into(),
                code: Some("upstream_timeout".into()),
            },
            Self::Upstream(msg) => OpenAiErrorDetail {
                message: msg.clone(),
                error_type: "server_error".into(),
                code: Some("internal_error".into()),
            },
            Self::Other(err) => OpenAiErrorDetail {
                message: err.to_string(),
                error_type: "server_error".into(),
                code: Some("internal_error".into()),
            },
            Self::Provider(err) => llm_error_detail(err),
        }
    }
}

fn llm_error_detail(err: &edgequake_llm::LlmError) -> OpenAiErrorDetail {
    match err {
        edgequake_llm::LlmError::RateLimited(msg) => OpenAiErrorDetail {
            message: msg.clone(),
            error_type: "rate_limit_error".into(),
            code: Some("rate_limit_exceeded".into()),
        },
        edgequake_llm::LlmError::AuthError(msg) => OpenAiErrorDetail {
            message: msg.clone(),
            error_type: "invalid_request_error".into(),
            code: Some("invalid_api_key".into()),
        },
        edgequake_llm::LlmError::ModelNotFound(msg) => OpenAiErrorDetail {
            message: msg.clone(),
            error_type: "invalid_request_error".into(),
            code: Some("model_not_found".into()),
        },
        edgequake_llm::LlmError::InvalidRequest(msg) => OpenAiErrorDetail {
            message: msg.clone(),
            error_type: "invalid_request_error".into(),
            code: None,
        },
        edgequake_llm::LlmError::ApiError(msg) | edgequake_llm::LlmError::ProviderError(msg)
            if is_overloaded(msg) =>
        {
            OpenAiErrorDetail {
                message: msg.clone(),
                error_type: "server_error".into(),
                code: Some("service_unavailable".into()),
            }
        }
        edgequake_llm::LlmError::Timeout => OpenAiErrorDetail {
            message: err.to_string(),
            error_type: "server_error".into(),
            code: Some("service_unavailable".into()),
        },
        other => OpenAiErrorDetail {
            message: other.to_string(),
            error_type: "server_error".into(),
            code: Some("internal_error".into()),
        },
    }
}

fn is_overloaded(msg: &str) -> bool {
    msg.contains("overloaded") || msg.contains("overloaded_error")
}

impl From<edgequake_llm::LlmError> for ProxyError {
    fn from(err: edgequake_llm::LlmError) -> Self {
        match &err {
            edgequake_llm::LlmError::RateLimited(_) => Self::RateLimited,
            edgequake_llm::LlmError::ModelNotFound(m) => Self::ModelNotFound(m.clone()),
            edgequake_llm::LlmError::AuthError(m) => Self::Unauthorized(m.clone()),
            edgequake_llm::LlmError::InvalidRequest(m) => Self::BadRequest(m.clone()),
            edgequake_llm::LlmError::ApiError(m) | edgequake_llm::LlmError::ProviderError(m)
                if is_overloaded(m) =>
            {
                Self::ServiceUnavailable
            }
            edgequake_llm::LlmError::Timeout => Self::ServiceUnavailable,
            _ => Self::Provider(err),
        }
    }
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = OpenAiErrorBody {
            error: self.openai_detail(),
        };
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;
    use edgequake_llm::LlmError;

    #[test]
    fn rate_limited_maps_to_429_openai_shape() {
        let err: ProxyError = LlmError::RateLimited("slow down".into()).into();
        assert_eq!(err.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            err.openai_detail().code.as_deref(),
            Some("rate_limit_exceeded")
        );
    }

    #[test]
    fn auth_error_maps_to_invalid_api_key() {
        let err: ProxyError = LlmError::AuthError("bad key".into()).into();
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(err.openai_detail().code.as_deref(), Some("invalid_api_key"));
    }

    #[test]
    fn model_not_found_maps_to_404() {
        let err: ProxyError = LlmError::ModelNotFound("nope".into()).into();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
        assert_eq!(err.openai_detail().code.as_deref(), Some("model_not_found"));
    }

    #[test]
    fn overloaded_api_error_maps_to_503() {
        let err: ProxyError = LlmError::ApiError("overloaded_error: servers busy".into()).into();
        assert_eq!(err.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            err.openai_detail().code.as_deref(),
            Some("service_unavailable")
        );
    }

    #[test]
    fn unauthorized_openai_type() {
        let detail = ProxyError::Unauthorized("x".into()).openai_detail();
        assert_eq!(detail.code.as_deref(), Some("invalid_api_key"));
    }

    #[test]
    fn into_response_preserves_status() {
        let err = ProxyError::ModelNotFound("unknown".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
