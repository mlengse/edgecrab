//! `edgecrab proxy` — polished CLI for the OpenAI-compatible inference bridge.

mod context;
mod doctor;
mod print;
mod setup;

use anyhow::{Context, Result, bail};
use edgecrab_proxy::{ProxyRunOptions, client_snippet, resolve_recipe, run_server};
use edgecrab_proxy::write_proxy_token;

use crate::cli_args::{ProxyCommand, ProxyTokenCommand};

pub use context::ProxySession;

pub async fn run(command: Option<ProxyCommand>) -> Result<()> {
    match command {
        None => run_overview(),
        Some(ProxyCommand::Start {
            host,
            port,
            allow_public,
            provider,
        }) => run_start(host, port, allow_public, provider).await,
        Some(ProxyCommand::Doctor) => doctor::run_doctor(),
        Some(ProxyCommand::Setup { provider, yes }) => setup::run_setup(provider, yes),
        Some(ProxyCommand::Enable { provider }) => setup::run_enable(provider),
        Some(ProxyCommand::Client { show_token }) => setup::run_client(show_token),
        Some(ProxyCommand::Status) => run_status(),
        Some(ProxyCommand::Upstreams) => run_upstreams(),
        Some(ProxyCommand::Token { command }) => run_token(command),
    }
}

fn run_overview() -> Result<()> {
    print::print_help();
    println!();
    let session = ProxySession::load()?;
    println!("{}", crate::proxy_hub::format_status(&session));
    println!("\nTip: /proxy in TUI  ·  edgecrab proxy setup grok");
    Ok(())
}

async fn run_start(
    host: String,
    port: u16,
    allow_public: bool,
    provider: Option<String>,
) -> Result<()> {
    let mut session = ProxySession::load()?;
    session.app.proxy.bind = host.clone();
    session.app.proxy.port = port;

    let forward_only = if let Some(p) = provider {
        let key = p.trim().to_ascii_lowercase();
        if let Some(recipe) = resolve_recipe(&key) {
            edgecrab_proxy::apply_recipe(&mut session.app.proxy, recipe);
            session.save_mut().ok();
        }
        if !session.proxy().forward_upstreams.contains_key(&key) {
            let available = session.upstream_keys().join(", ");
            bail!(
                "unknown forward upstream '{key}'. Run `edgecrab proxy enable {key}` or \
                 `edgecrab proxy setup`. Configured: [{available}]"
            );
        }
        session.app.proxy.default_forward_upstream = Some(key.clone());
        Some(key)
    } else if let Some(key) = session.proxy().default_forward_upstream.clone() {
        Some(key)
    } else {
        let keys = session.upstream_keys();
        if keys.len() == 1 {
            Some(keys[0].clone())
        } else {
            None
        }
    };

    if session.token_path.exists() {
        let _ = session.ensure_token()?;
    } else {
        let token = session.ensure_token_create()?;
        eprintln!("Created proxy token at {}", session.token_path.display());
        eprintln!("  Bearer {token}");
    }

    let preflight = forward_only
        .as_deref()
        .or(session.proxy().default_forward_upstream.as_deref());
    if let Some(key) = preflight {
        session.ensure_upstream_ready(key).await?;
    }

    let mode_hint = if let Some(ref key) = forward_only {
        format!("forward-only → {key} (model field ignored)")
    } else if !session.proxy().forward_upstreams.is_empty() {
        format!(
            "dual-mode: {} alias(es), {} forward upstream(s)",
            session.proxy().model_aliases.len(),
            session.proxy().forward_upstreams.len()
        )
    } else {
        "provider bridge (API keys from config — not OAuth forward)".into()
    };

    let snippet = client_snippet(
        session.proxy(),
        forward_only.as_deref().and_then(resolve_recipe),
        &session.ensure_token()?,
    );

    eprintln!(
        "\n╔══════════════════════════════════════════════════════╗\n\
         ║  EdgeCrab proxy                                      ║\n\
         ╚══════════════════════════════════════════════════════╝\n\
         \x20 Listen:  {}\n\
         \x20 Mode:    {mode_hint}\n\
         \x20 Token:   {}\n\
         \n\
         \x20 Client:  OPENAI_API_BASE={}\n\
         \x20          model=\"{}\" (or --provider mode)\n\
         \n\
         Press Ctrl+C to stop.",
        snippet.base_url,
        session.token_path.display(),
        snippet.base_url,
        snippet.model_alias,
    );

    let default_model_spec = session.default_model_spec();
    let token_path = session.token_path;
    let config = session.app.proxy;

    run_server(ProxyRunOptions {
        bind: host,
        port,
        allow_public,
        token_path,
        config,
        default_model_spec,
        forward_only,
    })
    .await
    .context("proxy server")
}

fn run_status() -> Result<()> {
    let session = ProxySession::load()?;
    println!("{}", crate::proxy_hub::format_status(&session));
    println!("TUI: /proxy  ·  CLI: edgecrab proxy doctor | setup grok | start --provider xai");
    Ok(())
}

fn run_upstreams() -> Result<()> {
    let session = ProxySession::load()?;
    let keys = session.upstream_keys();
    if keys.is_empty() {
        println!("No forward upstreams in config.\n");
        println!("Quick enable:");
        for line in edgecrab_proxy::builtin_upstream_catalog_lines() {
            println!("{line}");
        }
        return Ok(());
    }
    println!("Forward upstreams (Mode A):\n");
    for line in edgecrab_proxy::format_upstream_status_table(session.proxy()) {
        println!("{line}");
    }
    println!(
        "\nStart: edgecrab proxy start --provider <name>\n\
         Alias: proxy.model_aliases.<name> → forward:<name>"
    );
    Ok(())
}

fn run_token(command: ProxyTokenCommand) -> Result<()> {
    let session = ProxySession::load()?;
    let path = &session.token_path;
    match command {
        ProxyTokenCommand::Set { token } => {
            let value = write_proxy_token(path, token.as_deref())
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("Proxy token written to {}", path.display());
            println!("Use: Authorization: Bearer {value}");
        }
        ProxyTokenCommand::Show { show } => {
            if !path.exists() {
                bail!(
                    "no proxy token at {} — run `edgecrab proxy token set`",
                    path.display()
                );
            }
            if show {
                let value = edgecrab_proxy::load_proxy_token(path)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                println!("{value}");
            } else {
                println!(
                    "Proxy token at {} (use `edgecrab proxy token show --show`)",
                    path.display()
                );
            }
        }
        ProxyTokenCommand::Rotate => {
            let value = write_proxy_token(path, None).map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("Proxy token rotated at {}", path.display());
            println!("Use: Authorization: Bearer {value}");
        }
    }
    Ok(())
}
