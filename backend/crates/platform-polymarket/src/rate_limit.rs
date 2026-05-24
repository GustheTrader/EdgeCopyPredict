use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::trace;

/// Simple token-bucket-style rate limiter.
#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<RateLimiterInner>>,
}

struct RateLimiterInner {
    tokens: f64,
    last_refill: Instant,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
}

impl RateLimiter {
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RateLimiterInner {
                tokens: max_tokens,
                last_refill: Instant::now(),
                max_tokens,
                refill_rate,
            })),
        }
    }

    /// Wait until a token is available, then consume it.
    pub async fn acquire(&self, _description: &str) {
        loop {
            let mut state = self.inner.lock().await;
            let now = Instant::now();
            let elapsed = now.duration_since(state.last_refill).as_secs_f64();
            state.tokens = (state.tokens + elapsed * state.refill_rate).min(state.max_tokens);
            state.last_refill = now;

            if state.tokens >= 1.0 {
                state.tokens -= 1.0;
                trace!("rate_limiter acquired token for {}", _description);
                return;
            }

            let wait_secs: f64 = (1.0 - state.tokens) / state.refill_rate;
            drop(state);
            trace!(
                "rate_limiter waiting {:.2}s for {}",
                wait_secs,
                _description
            );
            tokio::time::sleep(Duration::from_secs_f64(wait_secs.max(0.01))).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn acquire_does_not_block_when_tokens_available() {
        let limiter = RateLimiter::new(10.0, 1.0);
        limiter.acquire("test").await;
        // Should complete immediately
    }

    #[tokio::test]
    async fn acquire_waits_when_tokens_exhausted() {
        let limiter = RateLimiter::new(1.0, 10.0);
        limiter.acquire("first").await;
        let start = Instant::now();
        limiter.acquire("second").await;
        assert!(start.elapsed() >= Duration::from_millis(50));
    }
}
