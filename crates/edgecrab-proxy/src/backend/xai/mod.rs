//! xAI Grok OAuth upstream (Hermes `hermes_cli/proxy/adapters/xai.py`).

mod adapter;
mod oauth_login;
mod refresh;

pub use adapter::XaiGrokAdapter;
pub use oauth_login::{
    login_xai_oauth, persist_xai_oauth, XaiOAuthAuthorizePrompt, XaiOAuthLoginOptions,
    XAI_OAUTH_PROVIDER,
};
pub use refresh::{DEFAULT_XAI_API, XAI_OAUTH_CLIENT_ID};
