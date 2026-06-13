//! Guided proxy setup (`edgecrab proxy setup [grok|nous|xai]`).

use std::io::{self, IsTerminal};

use anyhow::{Context, Result, bail};
use dialoguer::{Confirm, Select, theme::ColorfulTheme};

use edgecrab_proxy::{ALL_RECIPES, BuiltinRecipe, apply_recipe, resolve_recipe};

use crate::proxy_hub::{client_report, enable_by_name, format_recipe_auth_line};

use super::context::ProxySession;

fn theme() -> ColorfulTheme {
    ColorfulTheme::default()
}

fn pick_recipe(provider: Option<String>) -> Result<&'static BuiltinRecipe> {
    if let Some(name) = provider {
        return resolve_recipe(&name)
            .with_context(|| format!("unknown provider '{name}' (try: grok, xai, nous)"));
    }
    if !io::stdin().is_terminal() {
        bail!("non-interactive shell: pass provider, e.g. `edgecrab proxy setup grok`");
    }
    let items: Vec<String> = ALL_RECIPES
        .iter()
        .map(|r| format!("{} — {}", r.key, r.display_name))
        .collect();
    let idx = Select::with_theme(&theme())
        .with_prompt("Which subscription upstream?")
        .items(&items)
        .default(0)
        .interact()?;
    Ok(&ALL_RECIPES[idx])
}

pub fn run_setup(provider: Option<String>, yes: bool) -> Result<()> {
    let recipe = pick_recipe(provider)?;
    let mut session = ProxySession::load()?;

    println!("\nSetup: {}\n", recipe.display_name);
    println!("{}", format_recipe_auth_line(recipe));

    if !yes && io::stdin().is_terminal() {
        let proceed = Confirm::with_theme(&theme())
            .with_prompt(format!(
                "Add `{}` to {} and ensure proxy token?",
                recipe.key,
                ProxySession::config_path().display()
            ))
            .default(true)
            .interact()?;
        if !proceed {
            println!("Cancelled.");
            return Ok(());
        }
    }

    apply_recipe(&mut session.app.proxy, recipe);
    session.save_mut()?;
    println!(
        "\n  ✓ Saved upstream `{}` and alias `{}` → forward:{}",
        recipe.key, recipe.default_alias, recipe.key
    );

    let _token = session.ensure_token_create()?;
    println!("  ✓ Proxy token ready at {}", session.token_path.display());

    println!("{}", client_report(&session, true)?);

    println!("\nNext:");
    println!("  edgecrab proxy doctor");
    println!("  edgecrab proxy start --provider {}", recipe.key);
    Ok(())
}

pub fn run_enable(provider: String) -> Result<()> {
    let recipe = resolve_recipe(&provider)
        .with_context(|| format!("unknown provider '{provider}' (try: grok, xai, nous)"))?;
    let mut session = ProxySession::load()?;
    println!("{}", enable_by_name(&mut session, &provider)?);
    println!("{}", format_recipe_auth_line(recipe));
    Ok(())
}

pub fn run_client(show_token: bool) -> Result<()> {
    let session = ProxySession::load()?;
    println!("{}", client_report(&session, show_token)?);
    Ok(())
}
