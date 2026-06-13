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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_and_set_round_trip() {
        set_spawn_paused(false);
        assert!(!is_spawn_paused());
        assert!(set_spawn_paused(true));
        assert!(is_spawn_paused());
        assert!(!toggle_spawn_paused());
        assert!(!is_spawn_paused());
    }
}
