//! ChatGPT Pro / OpenAI Codex device OAuth (Hermes `_codex_device_code_login` parity).

use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde_json::{json, Value};
use tokio::time::sleep;

use super::auth_store::{
    default_auth_path, read_provider_access_token, read_provider_refresh_token,
    remove_provider_state, write_provider_state,
};

pub const OPENAI_CODEX_PROVIDER: &str = "openai-codex";
pub const CODEX_OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub const CODEX_OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
pub const CODEX_OAUTH_ISSUER: &str = "https://auth.openai.com";
pub const DEFAULT_CODEX_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";

pub const OPENAI_CODEX_ALIASES: &[&str] = &[
    "openai-codex",
    "chatgpt-pro",
    "chatgpt_pro",
    "codex",
];

#[derive(Clone, Default)]
pub struct CodexDeviceLoginOptions {
    pub on_device_code: Option<Arc<dyn Fn(CodexDevicePrompt) + Send + Sync>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexDevicePrompt {
    pub sign_in_url: String,
    pub user_code: String,
}

pub fn is_openai_codex_alias(target: &str) -> bool {
    let t = target.to_ascii_lowercase();
    OPENAI_CODEX_ALIASES.contains(&t.as_str())
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn codex_has_credentials(path: &std::path::Path) -> bool {
    read_provider_access_token(path, OPENAI_CODEX_PROVIDER)
        .ok()
        .flatten()
        .is_some()
}

pub async fn refresh_codex_oauth(refresh_token: &str) -> Result<Value, String> {
    if refresh_token.trim().is_empty() {
        return Err("Codex OAuth: missing refresh_token — run `edgecrab auth add chatgpt-pro`".into());
    }
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let resp = client
        .post(CODEX_OAUTH_TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", CODEX_OAUTH_CLIENT_ID),
        ])
        .send()
        .await
        .map_err(|e| format!("codex refresh: {e}"))?;
    let status = resp.status();
    let body = resp.text().await.map_err(|e| format!("codex refresh body: {e}"))?;
    if !status.is_success() {
        let relogin = status.as_u16() == 401 || status.as_u16() == 403;
        let hint = if relogin {
            " Re-login: edgecrab auth add chatgpt-pro"
        } else {
            ""
        };
        return Err(format!("codex refresh HTTP {}: {body}{hint}", status.as_u16()));
    }
    serde_json::from_str(&body).map_err(|e| format!("codex refresh JSON: {e}"))
}

pub async fn resolve_codex_access_token() -> Result<Option<String>, String> {
    let path = default_auth_path();
    if let Some(access) = read_provider_access_token(&path, OPENAI_CODEX_PROVIDER)? {
        return Ok(Some(access));
    }
    if let Some(refresh_token) = read_provider_refresh_token(&path, OPENAI_CODEX_PROVIDER)? {
        let payload = refresh_codex_oauth(&refresh_token).await?;
        if let Some(new_access) = payload.get("access_token").and_then(|v| v.as_str()) {
            let refresh = payload
                .get("refresh_token")
                .and_then(|v| v.as_str())
                .unwrap_or(&refresh_token);
            save_codex_tokens(
                &path,
                &json!({
                    "access_token": new_access,
                    "refresh_token": refresh,
                }),
            )?;
            return Ok(Some(new_access.to_string()));
        }
    }
    Ok(None)
}

fn save_codex_tokens(path: &std::path::Path, tokens: &Value) -> Result<(), String> {
    let state = json!({
        "tokens": tokens,
        "last_refresh": now_iso(),
        "auth_mode": "chatgpt",
        "base_url": DEFAULT_CODEX_BASE_URL,
    });
    write_provider_state(path, OPENAI_CODEX_PROVIDER, &state)
}

/// Force-refresh access token from persisted Codex OAuth state.
pub async fn refresh_codex_from_store() -> Result<String, String> {
    let path = default_auth_path();
    let Some(refresh_token) = read_provider_refresh_token(&path, OPENAI_CODEX_PROVIDER)? else {
        return Err("re-login required: /login chatgpt-pro".into());
    };
    let payload = refresh_codex_oauth(&refresh_token).await?;
    let access = payload
        .get("access_token")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "re-login required: /login chatgpt-pro".to_string())?;
    let refresh = payload
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .unwrap_or(&refresh_token);
    save_codex_tokens(
        &path,
        &json!({
            "access_token": access,
            "refresh_token": refresh,
        }),
    )?;
    Ok(access.to_string())
}

pub async fn login_codex_device_oauth(
    auth_path: Option<&std::path::Path>,
    opts: &CodexDeviceLoginOptions,
) -> Result<String, String> {
    let path = auth_path
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(default_auth_path);

    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("http client: {e}"))?;

    let resp = client
        .post(format!("{CODEX_OAUTH_ISSUER}/api/accounts/deviceauth/usercode"))
        .json(&json!({ "client_id": CODEX_OAUTH_CLIENT_ID }))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("device code request: {e}"))?;
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("device code body: {e}"))?;
    if !status.is_success() {
        return Err(format!("device code HTTP {}: {body}", status.as_u16()));
    }
    let device_data: Value =
        serde_json::from_str(&body).map_err(|e| format!("device code JSON: {e}"))?;
    let user_code = device_data
        .get("user_code")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let device_auth_id = device_data
        .get("device_auth_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if user_code.is_empty() || device_auth_id.is_empty() {
        return Err("device code response missing user_code or device_auth_id".into());
    }
    let poll_interval = device_data
        .get("interval")
        .and_then(|v| v.as_str().and_then(|s| s.parse().ok()).or_else(|| v.as_u64()))
        .unwrap_or(5)
        .max(3);

    let prompt = CodexDevicePrompt {
        sign_in_url: format!("{CODEX_OAUTH_ISSUER}/codex/device"),
        user_code: user_code.to_string(),
    };
    if let Some(hook) = &opts.on_device_code {
        hook(prompt.clone());
    } else {
        eprintln!("\nChatGPT Pro / Codex sign-in");
        eprintln!("===========================\n");
        eprintln!("1. Open: {}", prompt.sign_in_url);
        eprintln!("2. Enter code: {}\n", prompt.user_code);
        eprintln!("Waiting for sign-in...");
    }

    let max_wait = Duration::from_secs(15 * 60);
    let started = std::time::Instant::now();
    let mut code_resp: Option<Value> = None;
    while started.elapsed() < max_wait {
        sleep(Duration::from_secs(poll_interval)).await;
        let poll = client
            .post(format!("{CODEX_OAUTH_ISSUER}/api/accounts/deviceauth/token"))
            .json(&json!({
                "device_auth_id": device_auth_id,
                "user_code": user_code,
            }))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| format!("device poll: {e}"))?;
        let poll_status = poll.status();
        if poll_status.as_u16() == 200 {
            let text = poll.text().await.map_err(|e| format!("poll body: {e}"))?;
            code_resp = Some(
                serde_json::from_str(&text).map_err(|e| format!("poll JSON: {e}"))?,
            );
            break;
        }
        if poll_status.as_u16() == 403 || poll_status.as_u16() == 404 {
            continue;
        }
        let text = poll.text().await.unwrap_or_default();
        return Err(format!("device poll HTTP {}: {text}", poll_status.as_u16()));
    }
    let code_resp = code_resp.ok_or_else(|| {
        "Codex login timed out after 15 minutes — run `edgecrab auth add chatgpt-pro` again"
            .to_string()
    })?;

    let authorization_code = code_resp
        .get("authorization_code")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let code_verifier = code_resp
        .get("code_verifier")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if authorization_code.is_empty() || code_verifier.is_empty() {
        return Err("device auth missing authorization_code or code_verifier".into());
    }
    let redirect_uri = format!("{CODEX_OAUTH_ISSUER}/deviceauth/callback");
    let token_resp = client
        .post(CODEX_OAUTH_TOKEN_URL)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", authorization_code),
            ("redirect_uri", redirect_uri.as_str()),
            ("client_id", CODEX_OAUTH_CLIENT_ID),
            ("code_verifier", code_verifier),
        ])
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send()
        .await
        .map_err(|e| format!("token exchange: {e}"))?;
    let token_status = token_resp.status();
    let token_body = token_resp
        .text()
        .await
        .map_err(|e| format!("token body: {e}"))?;
    if !token_status.is_success() {
        return Err(format!(
            "token exchange HTTP {}: {token_body}",
            token_status.as_u16()
        ));
    }
    let tokens: Value =
        serde_json::from_str(&token_body).map_err(|e| format!("token JSON: {e}"))?;
    let access = tokens
        .get("access_token")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "token exchange missing access_token".to_string())?;
    let refresh = tokens
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    save_codex_tokens(
        &path,
        &json!({
            "access_token": access,
            "refresh_token": refresh,
        }),
    )?;
    Ok(format!(
        "ChatGPT Pro / Codex OAuth saved to {}. Use model chatgpt-pro/<model> when authenticated.",
        path.display()
    ))
}

pub fn remove_codex_oauth(path: Option<&std::path::Path>) -> Result<(), String> {
    let path = path
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(default_auth_path);
    remove_provider_state(&path, OPENAI_CODEX_PROVIDER)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_aliases_cover_chatgpt_pro() {
        assert!(is_openai_codex_alias("chatgpt-pro"));
        assert!(is_openai_codex_alias("openai-codex"));
    }

    #[test]
    fn save_and_probe_codex_credentials() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("auth.json");
        save_codex_tokens(
            &path,
            &json!({
                "access_token": "codex-access",
                "refresh_token": "codex-refresh",
            }),
        )
        .expect("save");
        assert!(codex_has_credentials(&path));
        remove_codex_oauth(Some(&path)).expect("remove");
        assert!(!codex_has_credentials(&path));
    }
}
