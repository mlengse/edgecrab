//! Human-readable checkpoint listing for CLI / gateway.

use std::path::Path;

use super::types::CheckpointEntry;

pub fn format_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = n as f64;
    for unit in UNITS {
        if size < 1024.0 || unit == "TB" {
            return if unit == "B" {
                format!("{n} {unit}")
            } else {
                format!("{size:.1} {unit}")
            };
        }
        size /= 1024.0;
    }
    format!("{size:.1} TB")
}

pub fn format_checkpoint_list(entries: &[CheckpointEntry], directory: &Path) -> String {
    if entries.is_empty() {
        return format!("No checkpoints found for {}", directory.display());
    }

    let mut lines = vec![format!("Checkpoints for {}:\n", directory.display())];
    for e in entries {
        let pin = if e.pinned { " [pinned]" } else { "" };
        let stat = if e.files_changed > 0 {
            format!(
                "  ({} file{}, +{}/{})",
                e.files_changed,
                if e.files_changed == 1 { "" } else { "s" },
                e.insertions,
                e.deletions
            )
        } else {
            String::new()
        };
        lines.push(format!(
            "  {}. {}  {}  {}{}  {}",
            e.n,
            e.short_hash,
            e.timestamp,
            e.reason,
            stat,
            format_bytes(e.size_bytes)
        ) + pin);
    }
    lines.push(String::new());
    lines.push("  /rollback <N>             restore to checkpoint N".into());
    lines.push("  /rollback diff <N>        preview changes since checkpoint N".into());
    lines.push("  /rollback pin <N>         pin checkpoint N (survives eviction)".into());
    lines.push("  /rollback <N> <file>      restore a single file from checkpoint N".into());
    lines.join("\n")
}
