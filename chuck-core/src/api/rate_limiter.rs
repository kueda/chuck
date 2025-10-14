use tokio::sync::{OnceCell, Mutex};
use tokio::time::{interval, Duration, Interval};

/// A centralized rate limiter for coordinating all iNaturalist API requests
/// Ensures we never exceed the 1 request per second rate limit
pub struct RateLimiter {
    interval: Mutex<Interval>,
}

impl RateLimiter {
    /// Create a new rate limiter with 1.1 second intervals for safety margin
    pub(crate) fn new() -> Self {
        let interval = interval(Duration::from_millis(1100));
        Self {
            interval: Mutex::new(interval),
        }
    }

    /// Wait for the next allowed request slot
    /// This method coordinates all API requests across the application
    pub async fn wait_for_next_request(&self) {
        let mut interval_guard = self.interval.lock().await;
        interval_guard.tick().await;
    }
}

// Global rate limiter instance shared across the entire application
static RATE_LIMITER: OnceCell<RateLimiter> = OnceCell::const_new();

/// Get the global rate limiter instance
pub async fn get_rate_limiter() -> &'static RateLimiter {
    RATE_LIMITER.get_or_init(|| async { RateLimiter::new() }).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Instant;

    #[tokio::test]
    async fn test_rate_limiter_enforces_timing() {
        // Create a new rate limiter for this test to avoid shared global state
        let rate_limiter = RateLimiter::new();

        let start = Instant::now();

        // First call should complete immediately (first tick is instant)
        rate_limiter.wait_for_next_request().await;
        let first_elapsed = start.elapsed();

        // Second call should wait for the interval
        rate_limiter.wait_for_next_request().await;
        let second_elapsed = start.elapsed();

        // Should be at least 1100ms between calls
        assert!(second_elapsed >= Duration::from_millis(1100));

        // First call should be relatively instant
        assert!(first_elapsed < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_multiple_callers_coordinate() {
        use std::sync::Arc;

        // Create a new rate limiter wrapped in Arc for shared ownership
        let rate_limiter = Arc::new(RateLimiter::new());

        // Simulate multiple concurrent callers
        let handles: Vec<_> = (0..3).map(|_| {
            let rate_limiter = Arc::clone(&rate_limiter);
            tokio::spawn(async move {
                rate_limiter.wait_for_next_request().await;
                Instant::now()
            })
        }).collect();

        let mut times = Vec::new();
        for handle in handles {
            times.push(handle.await.unwrap());
        }

        times.sort();

        // All requests should be spaced apart by at least the interval
        for i in 1..times.len() {
            let time_diff = times[i].duration_since(times[i-1]);
            // Allow some tolerance for scheduling variance
            assert!(time_diff >= Duration::from_millis(1050));
        }
    }
}
