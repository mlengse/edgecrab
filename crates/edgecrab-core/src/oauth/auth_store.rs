//! Hermes-format `auth.json` helpers (shared by Codex OAuth; proxy uses its own copy).

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::config::edgecrab_home;

pub fn default_auth_path() -> PathBuf {
    edgecrab_home().join("auth.json")
}

pub fn load_auth_doc(path: &Path) -> Result<Value, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))
}

pub fn write_auth_doc(path: &Path, doc: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(doc).map_err(|e| format!("serialize auth: {e}"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    std::fs::write(path, bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn read_provider_state(path: &Path, provider: &str) -> Result<Option<Value>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let doc = load_auth_doc(path)?;
    Ok(doc
        .get("providers")
        .and_then(|p| p.get(provider))
        .cloned())
}

pub fn write_provider_state(path: &Path, provider: &str, state: &Value) -> Result<(), String> {
    let mut doc = if path.exists() {
        load_auth_doc(path)?
    } else {
        Value::Object(Map::new())
    };
    let root = doc.as_object_mut().ok_or_else(|| "auth root must be object".to_string())?;
    let providers = root
        .entry("providers")
        .or_insert_with(|| Value::Object(Map::new()));
    if let Some(obj) = providers.as_object_mut() {
        obj.insert(provider.to_string(), state.clone());
    } else {
        return Err("auth providers field must be object".into());
    }
    write_auth_doc(path, &doc)
}

pub fn remove_provider_state(path: &Path, provider: &str) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let mut doc = load_auth_doc(path)?;
    if let Some(providers) = doc.get_mut("providers").and_then(|v| v.as_object_mut()) {
        providers.remove(provider);
    }
    if let Some(pool) = doc.get_mut("credential_pool").and_then(|v| v.as_object_mut()) {
        pool.remove(provider);
    }
    write_auth_doc(path, &doc)
}

pub fn read_provider_access_token(path: &Path, provider: &str) -> Result<Option<String>, String> {
    let Some(state) = read_provider_state(path, provider)? else {
        return Ok(None);
    };
    let token = state
        .get("tokens")
        .and_then(|t| t.get("access_token"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    Ok(token)
}

pub fn read_provider_refresh_token(path: &Path, provider: &str) -> Result<Option<String>, String> {
    let Some(state) = read_provider_state(path, provider)? else {
        return Ok(None);
    };
    Ok(state
        .get("tokens")
        .and_then(|t| t.get("refresh_token"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_provider_tokens_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("auth.json");
        let tokens = serde_json::json!({
            "access_token": "access-test",
            "refresh_token": "refresh-test",
        });
        let state = serde_json::json!({
            "tokens": tokens,
            "auth_mode": "chatgpt",
        });
        write_provider_state(&path, "openai-codex", &state).expect("write");
        let access = read_provider_access_token(&path, "openai-codex").expect("read");
        assert_eq!(access.as_deref(), Some("access-test"));
        remove_provider_state(&path, "openai-codex").expect("remove");
        assert!(read_provider_access_token(&path, "openai-codex")
            .expect("read")
            .is_none());
    }
}
