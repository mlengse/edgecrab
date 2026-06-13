//! Cross-process lock for Hermes-format `auth.json` (Hermes `_auth_store_lock`).

use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use crate::error::ProxyError;
use crate::http_client::e2e_direct_http_enabled;

fn lock_path(auth_path: &Path) -> PathBuf {
    let name = auth_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "auth.json".into());
    auth_path
        .parent()
        .map(|p| p.join(format!("{name}.lock")))
        .unwrap_or_else(|| PathBuf::from(format!("{name}.lock")))
}

/// Exclusive lock around auth store read/write (no-op on non-Unix).
pub fn with_auth_store_lock<T, F>(auth_path: &Path, f: F) -> Result<T, ProxyError>
where
    F: FnOnce() -> Result<T, ProxyError>,
{
    if e2e_direct_http_enabled() {
        return f();
    }
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;

        let lock = lock_path(auth_path);
        if let Some(parent) = lock.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ProxyError::UpstreamAuth(format!("create auth lock dir: {e}")))?;
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock)
            .map_err(|e| {
                ProxyError::UpstreamAuth(format!("open auth lock {}: {e}", lock.display()))
            })?;
        let fd = file.as_raw_fd();
        let rc = unsafe { libc::flock(fd, libc::LOCK_EX) };
        if rc != 0 {
            return Err(ProxyError::UpstreamAuth(
                "failed to acquire auth.json lock".into(),
            ));
        }
        let result = f();
        unsafe {
            libc::flock(fd, libc::LOCK_UN);
        }
        result
    }
    #[cfg(not(unix))]
    {
        f()
    }
}
