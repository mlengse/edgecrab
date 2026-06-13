//! Cross-platform session handoff — CLI → gateway platform transfer (Hermes parity).
//!
//! `/handoff <platform>` marks a CLI session as pending; the gateway watcher
//! re-binds the destination home channel to the CLI session_id and dispatches
//! a synthetic confirmation turn.

/// Usage text for `/handoff` (CLI only — Hermes parity).
pub const SESSION_HANDOFF_USAGE: &str = "Usage: /handoff <platform>\n\
Example: /handoff telegram\n\
Hands the current CLI session to that platform's home channel.\n\
The CLI exits on success; resume later with /resume.";

/// Returned when platform handoff is requested mid-turn.
pub const SESSION_HANDOFF_BUSY_MESSAGE: &str =
    "Agent is busy. Wait for the current turn to finish, then retry /handoff.";

/// Terminal states for the session handoff state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionHandoffState {
    Pending,
    Running,
    Completed,
    Failed,
}

impl SessionHandoffState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// Snapshot of platform handoff progress for a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionHandoffStatus {
    pub state: SessionHandoffState,
    pub platform: Option<String>,
    pub error: Option<String>,
}

/// Synthetic user message injected after CLI→platform session handoff.
pub fn format_session_handoff_synthetic_message(session_title: &str) -> String {
    format!(
        "[Session was just handed off from CLI (\"{session_title}\") to this \
         channel. The full prior conversation history is loaded above. Briefly \
         confirm you're working here and summarize what we were working on, so \
         the user can continue from this device.]"
    )
}

/// User-facing confirmation after a successful platform handoff (CLI).
pub fn format_session_handoff_cli_success(platform: &str, session_title: &str) -> String {
    format!(
        "↻ Handoff complete. The session is now active on {platform}.\n\
         Resume it on this CLI later with: /resume {session_title}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_handoff_state_roundtrip() {
        assert_eq!(
            SessionHandoffState::parse("pending"),
            Some(SessionHandoffState::Pending)
        );
        assert_eq!(SessionHandoffState::Pending.as_str(), "pending");
    }

    #[test]
    fn synthetic_message_includes_title() {
        let msg = format_session_handoff_synthetic_message("auth refactor");
        assert!(msg.contains("auth refactor"));
        assert!(msg.contains("handed off from CLI"));
    }
}
