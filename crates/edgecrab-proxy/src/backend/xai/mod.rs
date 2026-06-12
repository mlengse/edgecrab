//! xAI Grok OAuth upstream (Hermes `hermes_cli/proxy/adapters/xai.py`).

mod adapter;
mod oauth_login;
mod refresh;

pub use adapter::XaiGrokAdapter;
pub use oauth_login::{
    default_xai_pending_path, extract_xai_oauth_code_from_paste, finish_xai_oauth_login,
    login_xai_oauth, login_xai_oauth_finish, peek_xai_pending_session, persist_xai_oauth,
    start_xai_oauth_login, PENDING_SESSION_MAX_AGE_SECS, XaiOAuthAuthorizePrompt,
    XaiOAuthLoginOptions, XaiOAuthStarted, XAI_OAUTH_PROVIDER,
};
pub use refresh::{resolve_xai_credentials_async, DEFAULT_XAI_API, XAI_OAUTH_CLIENT_ID};
