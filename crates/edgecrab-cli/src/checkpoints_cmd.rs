//! `edgecrab checkpoints` — store visibility and maintenance.

use anyhow::{Context, Result, bail};
use edgecrab_core::edgecrab_home;
use edgecrab_tools::tools::checkpoint::{
    clear_all, clear_legacy, format_store_status, store_status,
};

use crate::cli_args::CheckpointsCommand;

pub fn run(command: Option<CheckpointsCommand>) -> Result<()> {
    match command.unwrap_or(CheckpointsCommand::Status { limit: 20 }) {
        CheckpointsCommand::Status { limit } => {
            let status = store_status(&edgecrab_home());
            print!("{}", format_store_status(&status, limit));
            Ok(())
        }
        CheckpointsCommand::Prune {
            retention_days,
            max_size_mb,
            keep_orphans,
        } => {
            println!("Pruning checkpoint store…");
            println!("  retention_days:    {retention_days}");
            println!("  delete_orphans:  {}", !keep_orphans);
            println!("  max_total_size_mb: {max_size_mb}");
            println!();
            let counts = edgecrab_tools::tools::checkpoint::prune_checkpoints(
                &edgecrab_home(),
                retention_days,
                !keep_orphans,
                max_size_mb,
            );
            println!("Scanned:         {}", counts.scanned);
            println!("Deleted orphan:  {}", counts.deleted_orphan);
            println!("Deleted stale:   {}", counts.deleted_stale);
            println!("Errors:          {}", counts.errors);
            println!(
                "Bytes reclaimed: {}",
                edgecrab_tools::tools::checkpoint::format_bytes(counts.bytes_freed)
            );
            Ok(())
        }
        CheckpointsCommand::Clear { force } => {
            let status = store_status(&edgecrab_home());
            if status.total_size_bytes == 0 && !status.base.exists() {
                println!("Nothing to clear — checkpoint base does not exist.");
                return Ok(());
            }
            println!(
                "This will delete the ENTIRE checkpoint base at {}",
                status.base.display()
            );
            println!(
                "  size:        {}",
                edgecrab_tools::tools::checkpoint::format_bytes(status.total_size_bytes)
            );
            println!("  projects:    {}", status.project_count);
            println!("  legacy dirs: {}", status.legacy_archives.len());
            println!();
            println!("All /rollback history for every working directory will be lost.");
            if !force && !confirm("Proceed?")? {
                bail!("Aborted.");
            }
            let result = clear_all(&edgecrab_home());
            if result.deleted {
                println!(
                    "Cleared. Reclaimed {}.",
                    edgecrab_tools::tools::checkpoint::format_bytes(result.bytes_freed)
                );
            } else {
                bail!("Could not clear checkpoint base.");
            }
            Ok(())
        }
        CheckpointsCommand::ClearLegacy { force } => {
            let status = store_status(&edgecrab_home());
            if status.legacy_archives.is_empty() {
                println!("No legacy archives to clear.");
                return Ok(());
            }
            let total: u64 = status.legacy_archives.iter().map(|a| a.size_bytes).sum();
            println!(
                "Found {} legacy archive(s), total {}:",
                status.legacy_archives.len(),
                edgecrab_tools::tools::checkpoint::format_bytes(total)
            );
            for arch in &status.legacy_archives {
                println!(
                    "  {:<40}  {:>10}",
                    arch.name,
                    edgecrab_tools::tools::checkpoint::format_bytes(arch.size_bytes)
                );
            }
            println!();
            println!("Legacy archives hold pre-v2 per-project shadow repos.");
            if !force && !confirm("Delete all legacy archives?")? {
                bail!("Aborted.");
            }
            let result = clear_legacy(&edgecrab_home());
            println!(
                "Deleted {} archive(s), reclaimed {}.",
                result.deleted,
                edgecrab_tools::tools::checkpoint::format_bytes(result.bytes_freed)
            );
            Ok(())
        }
    }
}

fn confirm(prompt: &str) -> Result<bool> {
    use std::io::{self, Write};
    print!("{prompt} [y/N]: ");
    io::stdout().flush().context("flush stdout")?;
    let mut line = String::new();
    io::stdin().read_line(&mut line).context("read stdin")?;
    Ok(matches!(
        line.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}
