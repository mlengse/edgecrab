#![deny(clippy::unwrap_used)]
#![cfg_attr(test, allow(clippy::unwrap_used))]

pub mod capability;
pub mod config;
pub mod delta;
pub mod diagnostics;
pub mod edit;
pub mod enrichment;
pub mod error;
pub mod gate;
pub mod manager;
pub mod position;
pub mod protocol;
pub mod range_shift;
pub mod render;
pub mod sync;
pub mod tools;

pub use diagnostics::DiagnosticCache;
pub use error::LspError;
pub use gate::EdgecrabLspGate;
pub use manager::{LspRuntime, runtime_for_ctx};
