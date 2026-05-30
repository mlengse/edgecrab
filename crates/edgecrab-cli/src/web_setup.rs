//! Web search / extract setup wizard — Hermes tools picker parity with richer UX.
//!
//! ```text
//! edgecrab setup web     ← multi-step CLI wizard
//! /web setup             ← same wizard from TUI (terminal handoff)
//! /web status            ← diagnostics overlay in TUI
//! ```

use std::io;
use std::path::Path;

use dialoguer::{Confirm, Input, Password, Select, theme::ColorfulTheme};
use serde_json::Value;

use crate::gateway_setup::save_env_key;

const BANNER: &str = r"
╔══════════════════════════════════════════════════════╗
║        EdgeCrab — Web Search & Extract Setup         ║
╚══════════════════════════════════════════════════════╝
";

/// Format a picker row label (name + badge + capabilities + configured marker).
pub fn format_picker_label(row: &Value, configured: bool) -> String {
    let name = row["name"].as_str().unwrap_or("Unknown");
    let badge = row["badge"].as_str().filter(|b| !b.is_empty());
    let caps = capability_suffix(row);
    let mut label = match badge {
        Some(b) => format!("{name}  [{b}]  {caps}"),
        None => format!("{name}  {caps}"),
    };
    if configured {
        label.push_str("  ✓");
    }
    label
}

fn capability_suffix(row: &Value) -> String {
    edgecrab_tools::capability_label(
        row["supports_search"].as_bool().unwrap_or(false),
        row["supports_extract"].as_bool().unwrap_or(false),
        row["supports_crawl"].as_bool().unwrap_or(false),
    )
}

fn wizard_theme() -> ColorfulTheme {
    ColorfulTheme::default()
}

fn dialoguer_err(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(e.to_string())
}

/// Rich status block for wizard header (shared with `/web` overlay).
pub fn status_report_text() -> String {
    edgecrab_tools::render_web_dashboard()
}

fn print_banner_and_status() {
    println!("{BANNER}");
    println!("{}", status_report_text());
    println!();
}

/// Interactive multi-step wizard — entry point for `edgecrab setup web`.
pub fn run(config_path: &Path) -> anyhow::Result<()> {
    loop {
        print_banner_and_status();
        let menu = [
            "Pick web backend (search / extract / both)",
            "Split: search backend vs extract backend",
            "Configure search fallback chain (web_search.primary + fallbacks)",
            "Reset to auto (clear web: overrides)",
            "Reset search chain to auto (clear web_search primary/fallbacks)",
            "Refresh status",
            "Done — exit wizard",
        ];
        let choice = Select::with_theme(&wizard_theme())
            .with_prompt("What would you like to configure?")
            .items(menu)
            .default(0)
            .interact()
            .map_err(dialoguer_err)?;

        match choice {
            0 => pick_backend_flow(config_path)?,
            1 => split_backend_flow(config_path)?,
            2 => configure_search_chain_flow(config_path)?,
            3 => {
                edgecrab_tools::clear_web_section_overrides(config_path)?;
                println!("\n  ✓ Cleared web.backend / search_backend / extract_backend");
                println!("  ✓ Auto fallback chain + native extract are active again.\n");
            }
            4 => {
                edgecrab_tools::clear_web_search_chain_in_config(config_path)?;
                println!("\n  ✓ Cleared web_search.primary / fallbacks");
                println!("  ✓ Legacy availability chain is active again.\n");
            }
            5 => {}
            _ => break,
        }
    }
    println!("\n✅ Web setup complete.");
    println!("   Run `edgecrab doctor` to verify search + extract readiness.");
    Ok(())
}

fn search_capable_backend_ids() -> Vec<String> {
    edgecrab_tools::web_provider_picker_rows()
        .into_iter()
        .filter(|row| row["supports_search"].as_bool() == Some(true))
        .filter_map(|row| row["id"].as_str().map(str::to_string))
        .collect()
}

fn configure_search_chain_flow(config_path: &Path) -> anyhow::Result<()> {
    let disk = edgecrab_tools::load_web_search_config_from_disk();
    println!(
        "\n  Current chain: {}\n",
        edgecrab_tools::format_search_chain_summary(&disk)
    );
    println!("  Note: web.search_backend / web.backend override this chain when set.\n");

    let backend_ids = search_capable_backend_ids();
    if backend_ids.is_empty() {
        anyhow::bail!("No search-capable backends registered");
    }

    let primary_labels: Vec<String> = backend_ids
        .iter()
        .map(|id| {
            let row = edgecrab_tools::web_provider_picker_rows()
                .into_iter()
                .find(|r| r["id"].as_str() == Some(id.as_str()));
            match row {
                Some(row) => {
                    format_picker_label(&row, row["configured"].as_bool().unwrap_or(false))
                }
                None => id.clone(),
            }
        })
        .collect();
    let primary_refs: Vec<&str> = primary_labels.iter().map(String::as_str).collect();

    let default_primary = backend_ids
        .iter()
        .position(|id| id == &disk.primary)
        .unwrap_or(0);
    let primary_idx = Select::with_theme(&wizard_theme())
        .with_prompt("Primary search backend (web_search.primary)")
        .items(&primary_refs)
        .default(default_primary)
        .interact()
        .map_err(dialoguer_err)?;
    let primary = backend_ids[primary_idx].clone();

    let current_fallbacks = disk.fallbacks.join(", ");
    let fallback_hint = if current_fallbacks.is_empty() {
        "brave, ddgs".to_string()
    } else {
        current_fallbacks.clone()
    };
    let fallback_input: String = Input::with_theme(&wizard_theme())
        .with_prompt("Fallback backends (comma-separated, tried in order)")
        .default(fallback_hint)
        .interact_text()
        .map_err(dialoguer_err)?;
    let fallbacks: Vec<String> = fallback_input
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();

    let timeout: u64 = Input::with_theme(&wizard_theme())
        .with_prompt("Request timeout (seconds)")
        .default(disk.timeout_secs.max(1))
        .interact_text()
        .map_err(dialoguer_err)?;

    edgecrab_tools::persist_web_search_chain_in_config(
        config_path,
        &edgecrab_tools::WebSearchChainUpdate {
            primary: Some(primary.clone()),
            fallbacks: Some(fallbacks.clone()),
            timeout_secs: Some(timeout),
        },
    )?;

    let summary = edgecrab_tools::format_search_chain_summary(
        &edgecrab_tools::load_web_search_config_from_path(config_path).unwrap_or(disk),
    );
    println!("\n  ✓ Saved search chain: {summary}");
    println!("  ✓ Config: {}\n", config_path.display());
    Ok(())
}

fn pick_backend_flow(config_path: &Path) -> anyhow::Result<()> {
    let rows = edgecrab_tools::web_provider_picker_rows();
    if rows.is_empty() {
        anyhow::bail!("No web search providers registered");
    }

    let labels: Vec<String> = rows
        .iter()
        .map(|row| {
            let configured = row["configured"].as_bool().unwrap_or(false);
            format_picker_label(row, configured)
        })
        .collect();
    let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();

    let idx = Select::with_theme(&wizard_theme())
        .with_prompt("Select web backend")
        .items(&label_refs)
        .default(0)
        .interact()
        .map_err(dialoguer_err)?;

    let row = &rows[idx];
    print_provider_detail(row);

    let backend_id = row["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("picker row missing id"))?;

    let supports_search = row["supports_search"].as_bool().unwrap_or(false);
    let supports_extract = row["supports_extract"].as_bool().unwrap_or(false);

    if !Confirm::with_theme(&wizard_theme())
        .with_prompt(format!("Use {backend_id} as configured?"))
        .default(true)
        .interact()
        .map_err(dialoguer_err)?
    {
        println!("  Cancelled.\n");
        return Ok(());
    }

    let routing = if supports_search && supports_extract {
        let options = [
            "Both search + extract (web.backend)",
            "Search only (web.search_backend)",
            "Extract only (web.extract_backend)",
        ];
        Select::with_theme(&wizard_theme())
            .with_prompt("Apply backend to which tools?")
            .items(options)
            .default(0)
            .interact()
            .map_err(dialoguer_err)?
    } else if supports_search {
        1
    } else if supports_extract {
        2
    } else {
        0
    };

    prompt_and_save_env_vars(row)?;

    let update = match routing {
        1 => edgecrab_tools::WebSectionUpdate {
            backend: Some(String::new()),
            search_backend: Some(backend_id.to_string()),
            extract_backend: None,
        },
        2 => edgecrab_tools::WebSectionUpdate {
            backend: Some(String::new()),
            search_backend: None,
            extract_backend: Some(backend_id.to_string()),
        },
        _ => edgecrab_tools::WebSectionUpdate {
            backend: Some(backend_id.to_string()),
            search_backend: Some(String::new()),
            extract_backend: Some(String::new()),
        },
    };

    edgecrab_tools::persist_web_section_in_config(config_path, &update)?;
    println!("\n  ✓ Saved web configuration to {}", config_path.display());
    Ok(())
}

fn split_backend_flow(config_path: &Path) -> anyhow::Result<()> {
    let rows = edgecrab_tools::web_provider_picker_rows();
    let search_rows: Vec<_> = rows
        .iter()
        .filter(|r| r["supports_search"].as_bool() == Some(true))
        .collect();
    let extract_rows: Vec<_> = rows
        .iter()
        .filter(|r| r["supports_extract"].as_bool() == Some(true))
        .collect();

    let search_labels: Vec<String> = search_rows
        .iter()
        .map(|r| format_picker_label(r, r["configured"].as_bool().unwrap_or(false)))
        .collect();
    let search_refs: Vec<&str> = search_labels.iter().map(String::as_str).collect();
    let search_idx = Select::with_theme(&wizard_theme())
        .with_prompt("Search backend (web.search_backend)")
        .items(&search_refs)
        .default(0)
        .interact()
        .map_err(dialoguer_err)?;
    let search_id = search_rows[search_idx]["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing search backend id"))?;

    let extract_labels: Vec<String> = extract_rows
        .iter()
        .map(|r| format_picker_label(r, r["configured"].as_bool().unwrap_or(false)))
        .collect();
    let extract_refs: Vec<&str> = extract_labels.iter().map(String::as_str).collect();
    let extract_idx = Select::with_theme(&wizard_theme())
        .with_prompt("Extract backend (web.extract_backend)")
        .items(&extract_refs)
        .default(0)
        .interact()
        .map_err(dialoguer_err)?;
    let extract_id = extract_rows[extract_idx]["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing extract backend id"))?;

    prompt_and_save_env_vars(search_rows[search_idx])?;
    if search_id != extract_id {
        prompt_and_save_env_vars(extract_rows[extract_idx])?;
    }

    edgecrab_tools::persist_web_section_in_config(
        config_path,
        &edgecrab_tools::WebSectionUpdate {
            backend: Some(String::new()),
            search_backend: Some(search_id.to_string()),
            extract_backend: Some(extract_id.to_string()),
        },
    )?;
    println!("\n  ✓ search={search_id}  extract={extract_id}\n");
    Ok(())
}

fn print_provider_detail(row: &Value) {
    let name = row["name"].as_str().unwrap_or("Unknown");
    let tag = row["tag"].as_str().unwrap_or("");
    let caps = capability_suffix(row);
    println!("\n  ── {name} [{caps}] ──");
    if !tag.is_empty() {
        println!("  {tag}");
    }
    if let Some(envs) = row["env_vars"].as_array() {
        for ev in envs {
            let key = ev["key"].as_str().unwrap_or("");
            if key.is_empty() {
                continue;
            }
            let set = std::env::var(key)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false);
            let mark = if set { "✓" } else { "·" };
            println!("  {mark} {key}");
        }
    }
    println!();
}

fn prompt_and_save_env_vars(row: &Value) -> anyhow::Result<()> {
    let Some(env_vars) = row["env_vars"].as_array() else {
        return Ok(());
    };
    if env_vars.is_empty() {
        println!("  ✓ No API keys required.");
        return Ok(());
    }

    for var in env_vars {
        let key = var["key"].as_str().unwrap_or_default();
        let prompt = var["prompt"].as_str().unwrap_or(key);
        if key.is_empty() {
            continue;
        }
        if std::env::var(key)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
        {
            println!("  ✓ {key} already set");
            continue;
        }
        if let Some(url) = var["url"].as_str().filter(|u| !u.is_empty()) {
            println!("  Get a key: {url}");
        }
        let value = Password::with_theme(&wizard_theme())
            .with_prompt(prompt)
            .interact()
            .map_err(dialoguer_err)?;
        let value = value.trim();
        if value.is_empty() {
            println!("  ⚠ Skipped {key} — add to ~/.edgecrab/.env later");
            continue;
        }
        save_env_key(key, value)?;
        unsafe {
            std::env::set_var(key, value);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_picker_label_includes_capabilities() {
        let row = serde_json::json!({
            "name": "Firecrawl",
            "badge": "paid",
            "configured": false,
            "supports_search": true,
            "supports_extract": true,
            "supports_crawl": true,
        });
        let label = format_picker_label(&row, false);
        assert!(label.contains("Firecrawl"));
        assert!(label.contains("S+E+C"));
    }

    #[test]
    fn status_report_includes_providers() {
        let text = status_report_text();
        assert!(text.contains("PROVIDERS"));
        assert!(text.contains("Search"));
    }
}
