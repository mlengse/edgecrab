//! Inject subscription OAuth tokens into process env before provider construction.

use super::anthropic::resolve_anthropic_oauth_access_token;
use super::codex::{DEFAULT_CODEX_BASE_URL, resolve_codex_access_token};

fn env_nonempty(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .is_some_and(|v| !v.trim().is_empty())
}

fn anthropic_key_from_env() -> bool {
    env_nonempty("ANTHROPIC_API_KEY") || env_nonempty("ANTHROPIC_AUTH_TOKEN")
}

fn openai_key_from_env() -> bool {
    env_nonempty("OPENAI_API_KEY")
}

/// Set `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` from OAuth stores when env is unset.
pub async fn inject_subscription_oauth_env(provider: &str) -> Result<(), String> {
    let canonical = provider.to_ascii_lowercase();
    match canonical.as_str() {
        "anthropic" | "claude-pro" | "claude" => {
            if !anthropic_key_from_env()
                && let Some(token) = resolve_anthropic_oauth_access_token().await?
            {
                // SAFETY: provider construction runs once per session startup.
                unsafe { std::env::set_var("ANTHROPIC_API_KEY", token) };
            }
        }
        "openai-codex" | "chatgpt-pro" | "codex" => {
            if !openai_key_from_env()
                && let Some(token) = resolve_codex_access_token().await?
            {
                unsafe { std::env::set_var("OPENAI_API_KEY", token) };
            }
        }
        _ => {}
    }
    Ok(())
}

/// Map Codex OAuth bearer into `openai-compatible` env vars (edgequake-llm factory).
pub fn prepare_openai_codex_compatible_env() {
    if !env_nonempty("OPENAI_COMPATIBLE_BASE_URL") {
        unsafe { std::env::set_var("OPENAI_COMPATIBLE_BASE_URL", DEFAULT_CODEX_BASE_URL) };
    }
    if !env_nonempty("OPENAI_COMPATIBLE_API_KEY")
        && let Ok(key) = std::env::var("OPENAI_API_KEY")
        && !key.trim().is_empty()
    {
        unsafe { std::env::set_var("OPENAI_COMPATIBLE_API_KEY", key.trim()) };
    }
}
