//! Claude Pro / Max OAuth (Hermes `run_hermes_oauth_login_pure` parity).
//!
//! Uses PKCE + pasted authorization code (not loopback). Credentials live in
//! `~/.edgecrab/.anthropic_oauth.json` (Hermes-compatible camelCase JSON).

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::config::edgecrab_home;

use super::pkce::{code_challenge, code_verifier};

pub const ANTHROPIC_OAUTH_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
pub const ANTHROPIC_OAUTH_AUTHORIZE_URL: &str = "https://claude.ai/oauth/authorize";
pub const ANTHROPIC_OAUTH_TOKEN_URL: &str = "https://console.anthropic.com/v1/oauth/token";
pub const ANTHROPIC_OAUTH_TOKEN_URL_ALT: &str = "https://platform.claude.com/v1/oauth/token";
pub const ANTHROPIC_OAUTH_REDIRECT_URI: &str = "https://console.anthropic.com/oauth/code/callback";
pub const ANTHROPIC_OAUTH_SCOPES: &str = "org:create_api_key user:profile user:inference";

const OAUTH_FILE_NAME: &str = ".anthropic_oauth.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnthropicOAuthFile {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(rename = "expiresAt")]
    pub expires_at_ms: i64,
}

pub type AnthropicAuthorizeHook = Arc<dyn Fn(&str) + Send + Sync>;

#[derive(Clone, Default)]
pub struct AnthropicOAuthLoginOptions {
    pub open_browser: bool,
    pub on_authorize: Option<AnthropicAuthorizeHook>,
}

fn oauth_file_path() -> PathBuf {
    edgecrab_home().join(OAUTH_FILE_NAME)
}

pub fn anthropic_oauth_path() -> PathBuf {
    oauth_file_path()
}

pub fn read_anthropic_oauth_file() -> Result<Option<AnthropicOAuthFile>, String> {
    let path = oauth_file_path();
    if !path.exists() {
        return Ok(None);
    }
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let data: Value =
        serde_json::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))?;
    let access = data
        .get("accessToken")
        .or_else(|| data.get("access_token"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if access.is_empty() {
        return Ok(None);
    }
    let refresh = data
        .get("refreshToken")
        .or_else(|| data.get("refresh_token"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let expires_at_ms = data
        .get("expiresAt")
        .or_else(|| data.get("expires_at_ms"))
        .and_then(|v| v.as_i64().or_else(|| v.as_u64().map(|n| n as i64)))
        .unwrap_or(0);
    Ok(Some(AnthropicOAuthFile {
        access_token: access.to_string(),
        refresh_token: refresh,
        expires_at_ms,
    }))
}

pub fn write_anthropic_oauth_file(creds: &AnthropicOAuthFile) -> Result<(), String> {
    let path = oauth_file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let payload = json!({
        "accessToken": creds.access_token,
        "refreshToken": creds.refresh_token,
        "expiresAt": creds.expires_at_ms,
    });
    let bytes = serde_json::to_vec_pretty(&payload).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(&path, bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn remove_anthropic_oauth_file() -> Result<(), String> {
    let path = oauth_file_path();
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("remove {}: {e}", path.display()))?;
    }
    Ok(())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn token_is_expiring(creds: &AnthropicOAuthFile, skew_ms: i64) -> bool {
    if creds.expires_at_ms <= 0 {
        return false;
    }
    now_ms() >= creds.expires_at_ms - skew_ms
}

pub async fn refresh_anthropic_oauth(refresh_token: &str) -> Result<AnthropicOAuthFile, String> {
    if refresh_token.is_empty() {
        return Err("Anthropic OAuth: missing refresh_token".into());
    }
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let form = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", ANTHROPIC_OAUTH_CLIENT_ID),
    ];
    let mut last_err = None;
    for endpoint in [ANTHROPIC_OAUTH_TOKEN_URL, ANTHROPIC_OAUTH_TOKEN_URL_ALT] {
        let resp = client
            .post(endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header(
                "User-Agent",
                "claude-cli/2.1.63 (external, cli; claude-vscode/1.0.13)",
            )
            .form(&form)
            .send()
            .await;
        match resp {
            Ok(r) => {
                let status = r.status();
                let body = r.text().await.map_err(|e| format!("refresh body: {e}"))?;
                if !status.is_success() {
                    last_err = Some(format!("refresh HTTP {}: {body}", status.as_u16()));
                    continue;
                }
                let payload: Value =
                    serde_json::from_str(&body).map_err(|e| format!("refresh JSON: {e}"))?;
                let access = payload
                    .get("access_token")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| format!("refresh missing access_token: {body}"))?;
                let refresh = payload
                    .get("refresh_token")
                    .and_then(|v| v.as_str())
                    .unwrap_or(refresh_token)
                    .to_string();
                let expires_in = payload
                    .get("expires_in")
                    .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|n| n as u64)))
                    .unwrap_or(3600);
                return Ok(AnthropicOAuthFile {
                    access_token: access.to_string(),
                    refresh_token: refresh,
                    expires_at_ms: now_ms() + (expires_in as i64) * 1000,
                });
            }
            Err(e) => last_err = Some(format!("refresh request: {e}")),
        }
    }
    Err(last_err.unwrap_or_else(|| "Anthropic refresh failed".into()))
}

/// Force-refresh access token from persisted OAuth state.
pub async fn refresh_anthropic_from_store() -> Result<String, String> {
    let Some(current) = read_anthropic_oauth_file()? else {
        return Err("re-login required: /login claude-pro".into());
    };
    if current.refresh_token.trim().is_empty() {
        return Err("re-login required: /login claude-pro".into());
    }
    let refreshed = refresh_anthropic_oauth(&current.refresh_token).await?;
    let access = refreshed.access_token.clone();
    write_anthropic_oauth_file(&refreshed)?;
    Ok(access)
}

/// Return a usable access token, refreshing when near expiry.
pub async fn resolve_anthropic_oauth_access_token() -> Result<Option<String>, String> {
    let Some(mut creds) = read_anthropic_oauth_file()? else {
        return Ok(None);
    };
    if token_is_expiring(&creds, 60_000) && !creds.refresh_token.is_empty() {
        creds = refresh_anthropic_oauth(&creds.refresh_token).await?;
        write_anthropic_oauth_file(&creds)?;
    }
    if creds.access_token.is_empty() {
        return Ok(None);
    }
    Ok(Some(creds.access_token))
}

fn build_authorize_url(challenge: &str, state: &str) -> String {
    let mut url = url::Url::parse(ANTHROPIC_OAUTH_AUTHORIZE_URL).expect("authorize base");
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("code", "true");
        q.append_pair("client_id", ANTHROPIC_OAUTH_CLIENT_ID);
        q.append_pair("response_type", "code");
        q.append_pair("redirect_uri", ANTHROPIC_OAUTH_REDIRECT_URI);
        q.append_pair("scope", ANTHROPIC_OAUTH_SCOPES);
        q.append_pair("code_challenge", challenge);
        q.append_pair("code_challenge_method", "S256");
        q.append_pair("state", state);
    }
    url.to_string()
}

async fn exchange_code(
    client: &Client,
    code: &str,
    state: &str,
    verifier: &str,
) -> Result<AnthropicOAuthFile, String> {
    let body = json!({
        "grant_type": "authorization_code",
        "client_id": ANTHROPIC_OAUTH_CLIENT_ID,
        "code": code,
        "state": state,
        "redirect_uri": ANTHROPIC_OAUTH_REDIRECT_URI,
        "code_verifier": verifier,
    });
    let resp = client
        .post(ANTHROPIC_OAUTH_TOKEN_URL)
        .header("Content-Type", "application/json")
        .header(
            "User-Agent",
            "claude-cli/2.1.63 (external, cli; claude-vscode/1.0.13)",
        )
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("token exchange: {e}"))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("token exchange body: {e}"))?;
    if !status.is_success() {
        return Err(format!("token exchange HTTP {}: {text}", status.as_u16()));
    }
    let payload: Value =
        serde_json::from_str(&text).map_err(|e| format!("token exchange JSON: {e}"))?;
    let access = payload
        .get("access_token")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "token exchange missing access_token".to_string())?;
    let refresh = payload
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let expires_in = payload
        .get("expires_in")
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|n| n as u64)))
        .unwrap_or(3600);
    Ok(AnthropicOAuthFile {
        access_token: access.to_string(),
        refresh_token: refresh,
        expires_at_ms: now_ms() + (expires_in as i64) * 1000,
    })
}

fn prompt_auth_code() -> Result<String, String> {
    eprintln!("After authorizing in the browser, paste the authorization code:");
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| format!("stdin: {e}"))?;
    Ok(line.trim().to_string())
}

fn parse_auth_code_input(raw: &str, expected_state: &str) -> Result<(String, String), String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("empty authorization code".into());
    }
    let parts: Vec<&str> = trimmed.split('#').collect();
    let code = parts[0].to_string();
    let state = if parts.len() > 1 {
        parts[1].to_string()
    } else {
        expected_state.to_string()
    };
    if state != expected_state {
        return Err("OAuth state mismatch — run login again".into());
    }
    Ok((code, state))
}

/// Run Claude Pro OAuth login and persist credentials.
pub async fn login_anthropic_oauth(opts: &AnthropicOAuthLoginOptions) -> Result<String, String> {
    let verifier = code_verifier();
    let challenge = code_challenge(&verifier);
    let state = Uuid::new_v4().simple().to_string();
    let authorize_url = build_authorize_url(&challenge, &state);

    if let Some(hook) = &opts.on_authorize {
        hook(&authorize_url);
    } else {
        eprintln!("\nClaude Pro / Max OAuth");
        eprintln!("=====================\n");
        eprintln!("Open this URL in your browser:\n{authorize_url}\n");
        if opts.open_browser {
            open_browser(&authorize_url);
        }
    }

    let pasted = prompt_auth_code()?;
    let (code, received_state) = parse_auth_code_input(&pasted, &state)?;

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let creds = exchange_code(&client, &code, &received_state, &verifier).await?;
    write_anthropic_oauth_file(&creds)?;
    Ok(format!(
        "Claude Pro OAuth saved to {}. Set model to anthropic/claude-sonnet-4 or similar.",
        oauth_file_path().display()
    ))
}

fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).status();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(url).status();
    #[cfg(windows)]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_code_with_state_suffix() {
        let (code, state) = parse_auth_code_input("abc123#state-xyz", "state-xyz").expect("ok");
        assert_eq!(code, "abc123");
        assert_eq!(state, "state-xyz");
    }

    #[test]
    fn authorize_url_contains_pkce() {
        let url = build_authorize_url("challenge-test", "state-test");
        assert!(url.contains("code_challenge=challenge-test"));
        assert!(url.contains("client_id="));
    }
}
