//! Proxy backends — provider bridge (Mode B) and credential forwarder (Mode A).

pub mod adapter;
pub mod auth_file;
pub mod auth_lock;
pub mod auth_store;
pub mod factory;
pub mod forwarder;
pub mod nous;
pub mod provider;
pub mod xai;

pub use adapter::{StaticBearerAdapter, UpstreamAdapter, UpstreamCredential, describe_adapter};
pub use auth_store::provider_state_from_doc;
pub use factory::{build_forward_adapter, build_forward_adapters};
pub use forwarder::{ForwardInbound, build_forwarder_client, forward_request};
pub use provider::handle_chat_completion;
