//! Per-backend token-bucket rate limiter (governor-style, no external crate).

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Simple token bucket for per-backend RPS caps.
#[derive(Debug)]
pub struct TokenBucket {
    capacity: f64,
    tokens: Mutex<(f64, Instant)>,
}

impl TokenBucket {
    pub fn new(rps: f64) -> Self {
        let rps = rps.max(0.1);
        Self {
            capacity: rps,
            tokens: Mutex::new((rps, Instant::now())),
        }
    }

    /// Returns true when a token was consumed; false when budget is exhausted.
    pub fn try_consume(&self) -> bool {
        let mut guard = self.tokens.lock().expect("rate limit mutex");
        let (tokens, last) = &mut *guard;
        let now = Instant::now();
        let elapsed = now.duration_since(*last).as_secs_f64();
        *tokens = (*tokens + elapsed * self.capacity).min(self.capacity);
        *last = now;
        if *tokens >= 1.0 {
            *tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Seconds until the next token is likely available (for cooldown hints).
    pub fn cooldown_secs(&self) -> u64 {
        let guard = self.tokens.lock().expect("rate limit mutex");
        let (tokens, _) = *guard;
        if tokens >= 1.0 {
            0
        } else {
            ((1.0 - tokens) / self.capacity).ceil() as u64
        }
    }
}

/// Tracks per-backend buckets and post-429 cooldown windows.
#[derive(Debug, Default)]
pub struct RateLimiter {
    buckets: dashmap::DashMap<String, TokenBucket>,
    cooldown_until: dashmap::DashMap<String, Instant>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn configure(&self, backend: &str, rps: Option<f64>) {
        if let Some(rps) = rps.filter(|v| *v > 0.0) {
            self.buckets
                .insert(backend.to_string(), TokenBucket::new(rps));
        }
    }

    pub fn mark_rate_limited(&self, backend: &str, cooldown: Duration) {
        self.cooldown_until
            .insert(backend.to_string(), Instant::now() + cooldown);
    }

    pub fn is_available(&self, backend: &str) -> bool {
        if let Some(until) = self.cooldown_until.get(backend) {
            if Instant::now() < *until {
                return false;
            }
            self.cooldown_until.remove(backend);
        }
        match self.buckets.get(backend) {
            Some(bucket) => bucket.try_consume(),
            None => true,
        }
    }

    /// True when this backend has an explicit RPS token bucket (paid APIs).
    pub fn has_rps_bucket(&self, backend: &str) -> bool {
        self.buckets.contains_key(backend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_bucket_allows_burst_then_throttles() {
        let bucket = TokenBucket::new(2.0);
        assert!(bucket.try_consume());
        assert!(bucket.try_consume());
        assert!(!bucket.try_consume());
    }

    #[test]
    fn rate_limiter_skips_backend_on_cooldown() {
        let limiter = RateLimiter::new();
        limiter.mark_rate_limited("brave", Duration::from_secs(60));
        assert!(!limiter.is_available("brave"));
    }
}
