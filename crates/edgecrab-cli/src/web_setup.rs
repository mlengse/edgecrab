//! Web search / extract setup wizard — shares logic with `/web` via edgecrab-tools::setup.
//!
//! ```text
//! edgecrab setup web     ← multi-step CLI wizard
//! /web                   ← in-TUI chain editor (web_setup_tui.rs)
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
            "Configure search priority chain",
            "Reset to auto (clear web + search chain overrides)",
            "Refresh status",
            "Done — exit wizard",
        ];
        let choice = Select::with_theme(&wizard_theme())
            .with_prompt("What would you like to configure?")
            .items(&menu)
            .default(0)
            .interact()
            .map_err(dialoguer_err)?;

        match choice {
            0 => pick_backend_flow(config_path)?,
            1 => split_backend_flow(config_path)?,
            2 => configure_search_chain_flow(config_path)?,
            3 => {
                edgecrab_tools::reset_web_to_auto(config_path)?;
                println!("\n  ✓ Auto mode — EdgeCrab picks the best configured backends.\n");
            }
            4 => {}
            _ => break,
        }
    }
    println!("\n✅ Web setup complete.");
    println!("   Run `edgecrab doctor` to verify search + extract readiness.");
    Ok(())
}

fn configure_search_chain_flow(config_path: &Path) -> anyhow::Result<()> {
    let mut editor = edgecrab_tools::WebChainEditor::load_from_disk();
    println!(
        "\n  Current chain ({}): {}\n",
        editor.mode_label(),
        editor.summary_arrow()
    );
    println!("  Tip: run `/web` in the TUI for visual reordering.\n");
    if let Some(w) = edgecrab_tools::search_override_warning() {
        println!("  {w}\n");
    }

    let default_chain = editor.order.join(", ");
    let chain_input: String = Input::with_theme(&wizard_theme())
        .with_prompt("Priority order (comma-separated, tried left → right)")
        .default(default_chain)
        .interact_text()
        .map_err(dialoguer_err)?;

    let parsed: Vec<String> = chain_input
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();

    if parsed.is_empty() {
        anyhow::bail!("Chain must include at least one backend");
    }

    for id in &parsed {
        if !editor.catalog.chain_eligible_ids.iter().any(|e| e == id) && id != "ddgs" {
            anyhow::bail!(
                "Unknown or unconfigured backend '{id}' — add API keys or use /web to see eligible providers"
            );
        }
    }

    editor.order = parsed;
    editor.is_auto = false;

    let disk = edgecrab_tools::load_web_search_config_from_disk();
    let timeout: u64 = Input::with_theme(&wizard_theme())
        .with_prompt("Request timeout (seconds)")
        .default(disk.timeout_secs.max(1))
        .interact_text()
        .map_err(dialoguer_err)?;

    edgecrab_tools::persist_search_chain_with_timeout(config_path, &editor.order, timeout)?;

    let summary = edgecrab_tools::chain_summary_after_save(config_path);
    println!("\n  ✓ Saved search chain: {summary}");
    println!("  ✓ Config: {}\n", config_path.display());
    Ok(())
}

fn pick_backend_flow(config_path: &Path) -> anyhow::Result<()> {
    let catalog = edgecrab_tools::WebPickerCatalog::load();
    if catalog.rows.is_empty() {
        anyhow::bail!("No web search providers registered");
    }

    let labels: Vec<String> = catalog
        .rows
        .iter()
        .map(edgecrab_tools::format_picker_label)
        .collect();
    let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();

    let idx = Select::with_theme(&wizard_theme())
        .with_prompt("Select web backend")
        .items(&label_refs)
        .default(0)
        .interact()
        .map_err(dialoguer_err)?;

    let row = &catalog.rows[idx];
    edgecrab_tools::print_provider_detail_cli(row);

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
            "Both search + extract",
            "Search only (priority chain)",
            "Extract only (web.extract_backend)",
        ];
        Select::with_theme(&wizard_theme())
            .with_prompt("Apply backend to which tools?")
            .items(&options)
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

    match routing {
        1 => {
            edgecrab_tools::persist_search_backend_as_chain(config_path, backend_id)?;
        }
        2 => {
            edgecrab_tools::persist_web_section_in_config(
                config_path,
                &edgecrab_tools::WebSectionUpdate {
                    backend: Some(String::new()),
                    search_backend: Some(String::new()),
                    extract_backend: Some(backend_id.to_string()),
                },
            )?;
        }
        _ => {
            edgecrab_tools::persist_search_backend_as_chain(config_path, backend_id)?;
            edgecrab_tools::persist_web_section_in_config(
                config_path,
                &edgecrab_tools::WebSectionUpdate {
                    backend: Some(String::new()),
                    search_backend: Some(String::new()),
                    extract_backend: Some(backend_id.to_string()),
                },
            )?;
        }
    }

    println!("\n  ✓ Saved web configuration to {}", config_path.display());
    Ok(())
}

fn split_backend_flow(config_path: &Path) -> anyhow::Result<()> {
    let catalog = edgecrab_tools::WebPickerCatalog::load();
    let search_rows: Vec<_> = catalog
        .rows
        .iter()
        .filter(|r| r["supports_search"].as_bool() == Some(true))
        .collect();
    let extract_rows: Vec<_> = catalog
        .rows
        .iter()
        .filter(|r| r["supports_extract"].as_bool() == Some(true))
        .collect();

    let search_labels: Vec<String> = search_rows
        .iter()
        .map(|r| edgecrab_tools::format_picker_label(r))
        .collect();
    let search_refs: Vec<&str> = search_labels.iter().map(String::as_str).collect();
    let search_idx = Select::with_theme(&wizard_theme())
        .with_prompt("Search backend (priority chain)")
        .items(&search_refs)
        .default(0)
        .interact()
        .map_err(dialoguer_err)?;
    let search_id = search_rows[search_idx]["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing search backend id"))?;

    let extract_labels: Vec<String> = extract_rows
        .iter()
        .map(|r| edgecrab_tools::format_picker_label(r))
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

    edgecrab_tools::persist_search_backend_as_chain(config_path, search_id)?;
    edgecrab_tools::persist_web_section_in_config(
        config_path,
        &edgecrab_tools::WebSectionUpdate {
            backend: Some(String::new()),
            search_backend: Some(String::new()),
            extract_backend: Some(extract_id.to_string()),
        },
    )?;
    let chain = edgecrab_tools::chain_summary_after_save(config_path);
    println!("\n  ✓ search chain: {chain}  extract={extract_id}\n");
    Ok(())
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
    fn status_report_includes_providers() {
        let text = status_report_text();
        assert!(text.contains("PROVIDERS"));
        assert!(text.contains("Search"));
    }
}
