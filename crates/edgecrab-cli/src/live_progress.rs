//! UI-side coalesce window for activity shelf redraws (Hermes `STREAM_BATCH_MS` parity).
//!
//! Agent events may arrive at ≤5/sec; the shelf only needs ~60fps max coalesced paint.

use std::time::{Duration, Instant};

/// Default coalesce interval for shelf layout updates.
pub const SHELF_COALESCE_MS: u64 = 16;

/// Returns true when the shelf should accept another paint after coalescing.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShelfCoalescer {
    last_paint: Option<Instant>,
    interval: Duration,
}

impl ShelfCoalescer {
    pub fn new() -> Self {
        Self {
            last_paint: None,
            interval: Duration::from_millis(SHELF_COALESCE_MS),
        }
    }

    pub fn should_paint(&mut self, now: Instant) -> bool {
        if self
            .last_paint
            .is_none_or(|prev| now.duration_since(prev) >= self.interval)
        {
            self.last_paint = Some(now);
            true
        } else {
            false
        }
    }

    pub fn force_paint(&mut self, now: Instant) {
        self.last_paint = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalescer_throttles_within_interval() {
        let mut c = ShelfCoalescer::new();
        let t0 = Instant::now();
        assert!(c.should_paint(t0));
        assert!(!c.should_paint(t0 + Duration::from_millis(1)));
        assert!(c.should_paint(t0 + Duration::from_millis(20)));
    }
}
