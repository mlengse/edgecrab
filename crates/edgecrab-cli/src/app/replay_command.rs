//! `/replay` slash command — in-memory nav + disk list/load (Hermes `ops.ts` parity).

use crate::transcript::OutputRole;

use super::App;

impl App {
    pub(super) fn handle_replay_command(&mut self, args: String) {
        let raw = args.trim();
        let lower = raw.to_lowercase();
        let turn_count = self.spawn_history.turn_count();

        if lower.starts_with("load ") {
            let path = raw[5..].trim();
            if path.is_empty() {
                self.push_output("usage: /replay load <path>", OutputRole::System);
                return;
            }
            match crate::spawn_tree_store::load_snapshot(path) {
                Ok(snapshot) => {
                    self.spawn_history.push_disk_snapshot(snapshot);
                    self.open_agents_replay(1);
                    self.push_output(format!("Loaded spawn tree from {path}"), OutputRole::System);
                }
                Err(err) => {
                    self.push_output(format!("replay load: {err}"), OutputRole::Error);
                }
            }
            return;
        }

        if lower == "list" || lower == "ls" {
            let session_id = self
                .current_session_key()
                .unwrap_or_else(|| "default".to_string());
            let disk = crate::spawn_tree_store::list_archived(&session_id, 30);
            if disk.is_empty() && turn_count == 0 {
                self.push_output(
                    "no archived spawn trees on disk for this session · try /agents after delegation",
                    OutputRole::System,
                );
                return;
            }

            let mut lines = vec!["Archived spawn trees:".to_string()];
            for entry in &disk {
                let ts =
                    chrono::DateTime::<chrono::Utc>::from_timestamp(entry.finished_at as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
                        .unwrap_or_else(|| "?".to_string());
                let label = if entry.label.is_empty() {
                    format!("{} subagents", entry.count)
                } else {
                    entry.label.clone()
                };
                lines.push(format!("  {ts} · {}× · {label}", entry.count));
                lines.push(format!("    {}", entry.path.display()));
            }
            if turn_count > 0 {
                lines.push(String::new());
                lines.push("In-memory this session:".into());
                for (i, turn) in self.spawn_history.turns().enumerate() {
                    lines.push(format!(
                        "  {} · {} delegate(s) · {} tools",
                        i + 1,
                        turn.delegate_count(),
                        turn.total_tools,
                    ));
                }
            }
            lines.push("Use `/replay N`, `/replay last`, or `/replay load <path>`.".into());
            self.push_output(lines.join("\n"), OutputRole::System);
            return;
        }

        if turn_count == 0 {
            self.push_output(
                "no completed spawn trees this session · try /replay list",
                OutputRole::System,
            );
            return;
        }

        let index = if raw.is_empty() || lower == "last" {
            1
        } else if let Ok(n) = raw.parse::<usize>() {
            if n < 1 || n > turn_count {
                self.push_output(
                    format!("replay: index out of range 1..{turn_count} · use /replay list"),
                    OutputRole::System,
                );
                return;
            }
            n
        } else {
            self.push_output(
                "usage: /replay [N|last|list|load <path>]",
                OutputRole::System,
            );
            return;
        };

        self.open_agents_replay(index);
    }
}
