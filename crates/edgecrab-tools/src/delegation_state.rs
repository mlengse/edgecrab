//! Process-wide delegation spawn controls — Hermes `delegate_tool.set_spawn_paused` parity.
//!
//! Blocks **new** `delegate_task` fan-out while active; running children continue.

use std::sync::atomic::{AtomicBool, Ordering};

static SPAWN_PAUSED: AtomicBool = AtomicBool::new(false);

/// Block or allow new subagent spawns. Returns the new paused state.
pub fn set_spawn_paused(paused: bool) -> bool {
    SPAWN_PAUSED.store(paused, Ordering::SeqCst);
    paused
}

pub fn is_spawn_paused() -> bool {
    SPAWN_PAUSED.load(Ordering::SeqCst)
}

/// Flip pause state; returns the new value.
pub fn toggle_spawn_paused() -> bool {
    set_spawn_paused(!is_spawn_paused())
}

/// When spawn is paused, returns the user-facing error message for `delegate_task`.
pub fn spawn_blocked_message() -> Option<String> {
    if is_spawn_paused() {
        Some(
            "Delegation spawning is paused. Clear the pause via the TUI (`p` in /agents) before retrying."
                .into(),
        )
    } else {
        None
    }
}

/// RAII restore for tests that flip global spawn pause (use with `serial_test`).
#[cfg(test)]
pub struct SpawnPauseGuard {
    previous: bool,
}

#[cfg(test)]
impl SpawnPauseGuard {
    pub fn set(paused: bool) -> Self {
        let previous = is_spawn_paused();
        set_spawn_paused(paused);
        Self { previous }
    }
}

#[cfg(test)]
impl Drop for SpawnPauseGuard {
    fn drop(&mut self) {
        set_spawn_paused(self.previous);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial(delegation_spawn_pause)]
    fn toggle_and_set_round_trip() {
        set_spawn_paused(false);
        assert!(!is_spawn_paused());
        assert!(set_spawn_paused(true));
        assert!(is_spawn_paused());
        assert!(!toggle_spawn_paused());
        assert!(!is_spawn_paused());
    }
}
