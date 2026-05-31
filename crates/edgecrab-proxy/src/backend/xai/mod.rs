//! xAI Grok OAuth upstream (Hermes `hermes_cli/proxy/adapters/xai.py`).

mod adapter;
mod refresh;

pub use adapter::XaiGrokAdapter;
pub use refresh::{DEFAULT_XAI_API, XAI_OAUTH_CLIENT_ID};
