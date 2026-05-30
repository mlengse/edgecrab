//! Session-scoped `computer_use` backends (one MCP session per conversation).

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use tokio::sync::Mutex;

use super::backend::ComputerUseBackend;
use super::cua_backend::CuaDriverBackend;
use super::noop::NoopBackend;

type BackendHandle = Arc<Mutex<Box<dyn ComputerUseBackend + Send>>>;

static POOL: OnceLock<Mutex<HashMap<String, BackendHandle>>> = OnceLock::new();

fn pool() -> &'static Mutex<HashMap<String, BackendHandle>> {
    POOL.get_or_init(|| Mutex::new(HashMap::new()))
}

fn backend_name() -> String {
    std::env::var("EDGECRAB_COMPUTER_USE_BACKEND")
        .unwrap_or_else(|_| "cua".to_string())
        .to_ascii_lowercase()
}

async fn create_backend(cua_cmd: &str) -> Result<BackendHandle, String> {
    let name = backend_name();
    let mut backend: Box<dyn ComputerUseBackend + Send> = if name == "noop" {
        Box::new(NoopBackend::new())
    } else {
        Box::new(CuaDriverBackend::new(cua_cmd))
    };
    backend.start().await?;
    Ok(Arc::new(Mutex::new(backend)))
}

/// Lazy per-session backend (starts MCP on first use).
pub async fn session_handle(session_id: &str, cua_cmd: &str) -> Result<BackendHandle, String> {
    let mut guard = pool().lock().await;
    if let Some(h) = guard.get(session_id) {
        return Ok(Arc::clone(h));
    }
    let handle = create_backend(cua_cmd).await?;
    guard.insert(session_id.to_string(), Arc::clone(&handle));
    Ok(handle)
}

/// Run `f` synchronously with this conversation's backend (lazy MCP start per `session_id`).
#[allow(dead_code)]
pub async fn with_session_backend<F, R>(session_id: &str, cua_cmd: &str, f: F) -> Result<R, String>
where
    F: FnOnce(&mut dyn ComputerUseBackend) -> R,
{
    let handle = session_handle(session_id, cua_cmd).await?;
    let mut guard = handle.lock().await;
    Ok(f(guard.as_mut()))
}

/// Test helper — drop all pooled backends.
#[cfg(test)]
pub async fn reset_pool_for_tests() {
    if let Some(p) = POOL.get() {
        p.lock().await.clear();
    }
}
