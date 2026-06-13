//! Nous Portal OAuth upstream (Hermes `hermes_cli/proxy/adapters/nous_portal.py`).

mod adapter;
pub mod device_flow;
mod inference_url;
mod jwt;
mod quarantine;
mod refresh;

pub use adapter::NousPortalAdapter;
pub use device_flow::{NousDeviceLoginOptions, login_nous_portal, persist_nous_oauth};
pub use jwt::{INFERENCE_INVOKE_SCOPE, make_jwt};
pub use quarantine::state_requires_relogin;
pub use refresh::{DEFAULT_NOUS_INFERENCE, DEFAULT_NOUS_PORTAL, resolve_nous_credentials_async};
