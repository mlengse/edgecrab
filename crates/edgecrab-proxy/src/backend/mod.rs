//! Proxy backends — provider bridge (Mode B) and credential forwarder (Mode A).

pub mod auth_lock;
pub mod adapter;
pub mod auth_file;
pub mod auth_store;
pub mod factory;
pub mod nous;
pub mod xai;
pub mod forwarder;
pub mod provider;

pub use adapter::{
    StaticBearerAdapter, UpstreamAdapter, UpstreamCredential, describe_adapter,
};
pub use auth_store::provider_state_from_doc;
pub use factory::{build_forward_adapter, build_forward_adapters};
pub use forwarder::{build_forwarder_client, forward_request, ForwardInbound};
pub use provider::handle_chat_completion;
