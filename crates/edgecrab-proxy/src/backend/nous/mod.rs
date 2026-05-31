//! Nous Portal OAuth upstream (Hermes `hermes_cli/proxy/adapters/nous_portal.py`).

mod adapter;
pub mod device_flow;
mod inference_url;
mod jwt;
mod quarantine;
mod refresh;

pub use adapter::NousPortalAdapter;
pub use jwt::{make_jwt, INFERENCE_INVOKE_SCOPE};
pub use quarantine::state_requires_relogin;
pub use device_flow::{login_nous_portal, persist_nous_oauth, NousDeviceLoginOptions};
pub use refresh::{resolve_nous_credentials_async, DEFAULT_NOUS_INFERENCE, DEFAULT_NOUS_PORTAL};
