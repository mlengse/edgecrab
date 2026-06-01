//! Shared proxy hub logic — DRY for `edgecrab proxy`, `/proxy`, and the setup TUI.

use std::fmt::Write as _;
use anyhow::{Context, Result};
use edgecrab_core::ProxyConfig;
use edgecrab_proxy::{
    ALL_RECIPES, AuthProbe, BuiltinRecipe, ClientSnippet, apply_recipe, auth_probe_message,
    builtin_upstream_catalog_lines, client_snippet, format_upstream_status_table, probe_oauth_auth,
    resolve_recipe,
};

use crate::proxy_cmd::ProxySession;

/// Accent for proxy overlays (cool cyan — distinct from `/web` amber).
pub const PROXY_ACCENT: ratatui::style::Color = ratatui::style::Color::Rgb(100, 200, 255);

pub fn usage() -> &'static str {
    "/proxy — OpenAI-compatible local bridge (Aider, Cline, OpenAI SDK)\n\
     \n\
     In TUI (default):\n\
       /proxy              open setup wizard (Grok / xAI, Nous)\n\
       /proxy setup        same wizard\n\
     \n\
     Text reports:\n\
       /proxy status       listen URL, aliases, upstreams\n\
       /proxy doctor       preflight (token + OAuth)\n\
       /proxy client       client env snippet (token redacted)\n\
       /proxy enable grok  enable preset without opening TUI\n\
     \n\
     CLI (foreground server):\n\
       edgecrab proxy start --provider xai\n\
     \n\
     Provider OAuth (once): edgecrab auth add grok\n\
     Clients use http://127.0.0.1:11434/v1 + proxy token (not provider key)."
}

pub fn format_status(session: &ProxySession) -> String {
    let cfg = session.proxy();
    let mut out = String::new();
    let _ = writeln!(
        out,
        "Proxy listen: http://{}:{}/v1",
        cfg.bind, cfg.port
    );
    let _ = writeln!(
        out,
        "Token: {} ({})",
        session.token_path.display(),
        if session.token_present() {
            "ready"
        } else {
            "missing — enable a preset in /proxy or run edgecrab proxy setup grok"
        }
    );
    let _ = writeln!(out, "Aliases ({}):", cfg.model_aliases.len());
    for (alias, spec) in &cfg.model_aliases {
        let _ = writeln!(out, "  {alias} → {spec}");
    }
    let lines = format_upstream_status_table(cfg);
    if lines.is_empty() {
        let _ = writeln!(out, "\nNo forward upstreams configured.");
        let _ = writeln!(out, "Built-in presets:");
        for line in builtin_upstream_catalog_lines() {
            let _ = writeln!(out, "{line}");
        }
    } else {
        let _ = writeln!(out, "\nForward upstreams:");
        for line in lines {
            let _ = writeln!(out, "{line}");
        }
    }
    let _ = writeln!(
        out,
        "\nStart: edgecrab proxy start --provider xai  (or model \"grok\" in dual-mode)"
    );
    out
}

pub fn format_doctor(session: &ProxySession) -> (u32, String) {
    let cfg = session.proxy();
    let mut issues = 0u32;
    let mut out = String::from("Proxy doctor\n\n");

    if session.token_present() {
        let _ = writeln!(
            out,
            "  ✓ proxy token at {}",
            session.token_path.display()
        );
    } else {
        let _ = writeln!(
            out,
            "  ✗ proxy token missing — run /proxy or `edgecrab proxy setup grok`"
        );
        issues += 1;
    }

    if cfg.forward_upstreams.is_empty() && cfg.model_aliases.is_empty() {
        let _ = writeln!(
            out,
            "  ○ no forward upstreams (run /proxy to enable Grok or Nous)"
        );
    }

    for line in format_upstream_status_table(cfg) {
        if line.contains("not ready") || line.contains("not authenticated") {
            issues += 1;
        }
        let _ = writeln!(out, "{line}");
    }

    if cfg.forward_upstreams.is_empty() {
        let _ = writeln!(out, "\nOAuth presets (auth.json):");
        for recipe in ALL_RECIPES {
            let probe = probe_oauth_auth(recipe);
            let icon = if probe == AuthProbe::Ready { "✓" } else { "○" };
            let _ = writeln!(out, "  {icon} {}", auth_probe_message(recipe, probe));
            if !matches!(probe, AuthProbe::Ready) {
                issues += 1;
            }
        }
    }

    let _ = writeln!(out);
    if issues == 0 {
        let _ = writeln!(out, "All checks passed. Start: edgecrab proxy start --provider xai");
    } else {
        let _ = writeln!(
            out,
            "{issues} issue(s). Try: /proxy  or  edgecrab proxy setup grok"
        );
    }
    (issues, out)
}

pub fn format_client_snippet_text(snippet: &ClientSnippet, redact_token: bool) -> String {
    let key = if redact_token {
        "(run /proxy client with token on disk, or edgecrab proxy token show --show)"
    } else {
        snippet.token.as_str()
    };
    format!(
        "Client configuration\n\
         Base URL:  {}\n\
         API key:   {key}  (local proxy token — not the provider key)\n\
         Model:     {}\n\
         \n\
         export OPENAI_API_BASE=\"{}\"\n\
         export OPENAI_API_KEY=\"{key}\"\n\
         \n\
         Aider (~/.aider.conf.yml):\n\
           openai-api-base: {}\n\
           openai-api-key: {key}\n\
         \n\
         Start server:\n\
         {}",
        snippet.base_url,
        snippet.model_alias,
        snippet.base_url,
        snippet.base_url,
        snippet.forward_only_cmd
    )
}

pub fn format_recipe_auth_line(recipe: &BuiltinRecipe) -> String {
    let probe = probe_oauth_auth(recipe);
    let icon = if probe == AuthProbe::Ready {
        "✓"
    } else {
        "○"
    };
    let hint = if probe == AuthProbe::Ready {
        String::new()
    } else {
        " — press a to sign in".to_string()
    };
    format!("{icon} {}{}", auth_probe_message(recipe, probe), hint)
}

/// TUI/CLI auth target for a forward preset (`/proxy` key `a`).
pub fn oauth_login_target(recipe: &BuiltinRecipe) -> Option<&'static str> {
    match recipe.key {
        "xai" => Some("grok"),
        "nous" => Some("nous"),
        _ => None,
    }
}

pub fn recipe_enabled_in_config(cfg: &ProxyConfig, recipe: &BuiltinRecipe) -> bool {
    cfg.forward_upstreams.contains_key(recipe.key)
        && cfg
            .model_aliases
            .get(recipe.default_alias)
            .is_some_and(|v| v == &format!("forward:{}", recipe.key))
}

/// Enable preset + proxy token (same as `edgecrab proxy setup <preset> --yes`).
pub fn enable_preset(session: &mut ProxySession, recipe: &BuiltinRecipe) -> Result<String> {
    apply_recipe(&mut session.app.proxy, recipe);
    session.save_mut()?;
    let token = session.ensure_token_create()?;
    let snippet = client_snippet(session.proxy(), Some(recipe), &token);
    Ok(format!(
        "Enabled {} — alias `{}` → forward:{}\n\
         Token: {}\n\
         Client base: {}\n\
         Start: {}",
        recipe.display_name,
        recipe.default_alias,
        recipe.key,
        session.token_path.display(),
        snippet.base_url,
        snippet.forward_only_cmd
    ))
}

pub fn enable_by_name(session: &mut ProxySession, name: &str) -> Result<String> {
    let recipe = resolve_recipe(name)
        .with_context(|| format!("unknown preset '{name}' (try: grok, xai, nous)"))?;
    enable_preset(session, recipe)
}

pub fn client_report(session: &ProxySession, show_token: bool) -> Result<String> {
    let token = if show_token {
        session.ensure_token_create()?
    } else if session.token_present() {
        session.ensure_token()?
    } else {
        let recipe = ALL_RECIPES.first();
        let snippet = client_snippet(session.proxy(), recipe, "(not set)");
        return Ok(format_client_snippet_text(&snippet, true));
    };
    let recipe = session
        .upstream_keys()
        .first()
        .and_then(|k| resolve_recipe(k))
        .or(ALL_RECIPES.first());
    let snippet = client_snippet(session.proxy(), recipe, &token);
    Ok(format_client_snippet_text(&snippet, !show_token))
}

pub fn listen_url(cfg: &ProxyConfig) -> String {
    format!("http://{}:{}/v1", cfg.bind, cfg.port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_mentions_proxy_tui_and_grok() {
        assert!(usage().contains("/proxy"));
        assert!(usage().contains("grok"));
    }

    #[test]
    fn resolve_grok_via_hub() {
        assert_eq!(resolve_recipe("grok").map(|r| r.key), Some("xai"));
    }

    #[test]
    fn oauth_login_targets_for_presets() {
        let grok = resolve_recipe("grok").expect("grok");
        assert_eq!(oauth_login_target(grok), Some("grok"));
        let nous = resolve_recipe("nous").expect("nous");
        assert_eq!(oauth_login_target(nous), Some("nous"));
    }
}
