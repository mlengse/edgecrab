//! Transcript scroll bounds — Hermes `limits.ts` `MAX_HISTORY` parity.

/// Maximum transcript lines retained in memory (older lines are dropped from the front).
pub const MAX_TRANSCRIPT_LINES: usize = 800;

/// How many lines to remove when `current_len` exceeds [`MAX_TRANSCRIPT_LINES`].
pub fn prune_count(current_len: usize) -> usize {
    current_len.saturating_sub(MAX_TRANSCRIPT_LINES)
}

/// Shift a line index after removing `removed` lines from the front of the transcript.
/// Returns `None` when the indexed line was pruned.
pub fn shift_line_index(index: usize, removed: usize) -> Option<usize> {
    if removed == 0 {
        return Some(index);
    }
    if index < removed {
        None
    } else {
        Some(index - removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prune_count_at_cap() {
        assert_eq!(prune_count(800), 0);
        assert_eq!(prune_count(850), 50);
    }

    #[test]
    fn shift_line_index_after_prune() {
        assert_eq!(shift_line_index(10, 5), Some(5));
        assert_eq!(shift_line_index(3, 5), None);
    }
}
