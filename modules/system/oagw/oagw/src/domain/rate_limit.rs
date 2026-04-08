use std::collections::HashSet;
use std::time::{Instant, SystemTime};

use crate::domain::error::DomainError;
use crate::domain::model::{RateLimitConfig, Window};
use dashmap::DashMap;
use modkit_macros::domain_model;

/// Quota metadata returned on successful token consumption.
#[domain_model]
#[derive(Debug, Clone, Copy)]
pub struct RateLimitOutcome {
    /// The bucket capacity (maps to `X-RateLimit-Limit`).
    pub limit: u64,
    /// Remaining tokens after consumption (maps to `X-RateLimit-Remaining`).
    pub remaining: u64,
    /// Unix epoch timestamp when the bucket will be full again (maps to `X-RateLimit-Reset`).
    pub reset_epoch: u64,
}

#[domain_model]
pub struct RateLimiter {
    buckets: DashMap<String, TokenBucket>,
}

#[domain_model]
struct TokenBucket {
    capacity: f64,
    tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    fn new(config: &RateLimitConfig) -> Self {
        let capacity = config
            .burst
            .as_ref()
            .map_or(config.sustained.rate as f64, |b| b.capacity as f64);
        let window_secs = window_to_secs(&config.sustained.window);
        let refill_rate = config.sustained.rate as f64 / window_secs;
        Self {
            capacity,
            tokens: capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }

    fn try_consume(&mut self, cost: f64) -> bool {
        self.refill();
        if self.tokens >= cost {
            self.tokens -= cost;
            true
        } else {
            false
        }
    }

    fn retry_after_secs(&self, cost: f64) -> u64 {
        if self.refill_rate <= 0.0 {
            return 60;
        }
        let needed = cost - self.tokens;
        if needed <= 0.0 {
            return 0;
        }
        (needed / self.refill_rate).ceil() as u64
    }
}

fn window_to_secs(window: &Window) -> f64 {
    match window {
        Window::Second => 1.0,
        Window::Minute => 60.0,
        Window::Hour => 3600.0,
        Window::Day => 86400.0,
    }
}

impl RateLimiter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            buckets: DashMap::new(),
        }
    }

    /// Remove all entries whose keys are not in `active_keys`.
    #[allow(dead_code)]
    pub fn purge_keys(&self, active_keys: &HashSet<String>) {
        self.buckets.retain(|k, _| active_keys.contains(k));
    }

    /// Remove a single rate-limit bucket by key.
    ///
    /// Called when an upstream or route is deleted so the stale bucket
    /// does not linger in memory.
    pub fn remove_key(&self, key: &str) {
        self.buckets.remove(key);
    }

    /// Try to consume tokens for the given key.
    ///
    /// # Errors
    /// Returns `DomainError::RateLimitExceeded` with Retry-After seconds when exhausted.
    pub fn try_consume(
        &self,
        key: &str,
        config: &RateLimitConfig,
        instance_uri: &str,
    ) -> Result<RateLimitOutcome, DomainError> {
        let cost = config.cost as f64;
        let mut bucket = self
            .buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(config));

        let now_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if bucket.try_consume(cost) {
            let limit = bucket.capacity as u64;
            let remaining = bucket.tokens.floor().max(0.0) as u64;
            let secs_to_full = if bucket.refill_rate > 0.0 {
                ((bucket.capacity - bucket.tokens) / bucket.refill_rate).ceil() as u64
            } else {
                0
            };
            Ok(RateLimitOutcome {
                limit,
                remaining,
                reset_epoch: now_epoch + secs_to_full,
            })
        } else {
            let retry_after = bucket.retry_after_secs(cost);
            let limit = bucket.capacity as u64;
            let remaining = bucket.tokens.floor().max(0.0) as u64;
            let secs_to_full = if bucket.refill_rate > 0.0 {
                ((bucket.capacity - bucket.tokens) / bucket.refill_rate).ceil() as u64
            } else {
                0
            };
            Err(DomainError::RateLimitExceeded {
                detail: format!("rate limit exceeded for key: {key}"),
                instance: instance_uri.to_string(),
                retry_after_secs: Some(retry_after),
                limit: if config.response_headers {
                    Some(limit)
                } else {
                    None
                },
                remaining: if config.response_headers {
                    Some(remaining)
                } else {
                    None
                },
                reset_epoch: if config.response_headers {
                    Some(now_epoch + secs_to_full)
                } else {
                    None
                },
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::model::{
        BurstConfig, RateLimitAlgorithm, RateLimitScope, RateLimitStrategy, SustainedRate,
    };

    use super::*;

    fn make_config(rate: u32, window: Window, burst_capacity: Option<u32>) -> RateLimitConfig {
        RateLimitConfig {
            sharing: Default::default(),
            algorithm: RateLimitAlgorithm::TokenBucket,
            sustained: SustainedRate { rate, window },
            burst: burst_capacity.map(|c| BurstConfig { capacity: c }),
            scope: RateLimitScope::Tenant,
            strategy: RateLimitStrategy::Reject,
            cost: 1,
            response_headers: true,
        }
    }

    #[test]
    fn allows_within_capacity() {
        let limiter = RateLimiter::new();
        let config = make_config(10, Window::Second, None);
        for _ in 0..10 {
            assert!(limiter.try_consume("test", &config, "/test").is_ok());
        }
    }

    #[test]
    fn denies_when_exhausted() {
        let limiter = RateLimiter::new();
        let config = make_config(2, Window::Second, None);
        assert!(limiter.try_consume("test", &config, "/test").is_ok());
        assert!(limiter.try_consume("test", &config, "/test").is_ok());
        let err = limiter.try_consume("test", &config, "/test").unwrap_err();
        assert!(matches!(err, DomainError::RateLimitExceeded { .. }));
    }

    #[test]
    fn retry_after_is_calculated() {
        let limiter = RateLimiter::new();
        let config = make_config(1, Window::Minute, None);
        assert!(limiter.try_consume("test", &config, "/test").is_ok());
        match limiter.try_consume("test", &config, "/test") {
            Err(DomainError::RateLimitExceeded {
                retry_after_secs, ..
            }) => {
                // ~60 seconds (1 token per minute).
                assert!(retry_after_secs.unwrap() > 0);
                assert!(retry_after_secs.unwrap() <= 60);
            }
            other => panic!("expected RateLimitExceeded, got {other:?}"),
        }
    }

    #[test]
    fn burst_capacity_used() {
        let limiter = RateLimiter::new();
        let config = make_config(1, Window::Second, Some(5));
        for _ in 0..5 {
            assert!(limiter.try_consume("test", &config, "/test").is_ok());
        }
        assert!(limiter.try_consume("test", &config, "/test").is_err());
    }

    #[test]
    fn separate_keys_independent() {
        let limiter = RateLimiter::new();
        let config = make_config(1, Window::Second, None);
        assert!(limiter.try_consume("key-a", &config, "/test").is_ok());
        assert!(limiter.try_consume("key-b", &config, "/test").is_ok());
        assert!(limiter.try_consume("key-a", &config, "/test").is_err());
        assert!(limiter.try_consume("key-b", &config, "/test").is_err());
    }

    #[test]
    fn purge_removes_stale_entries() {
        let limiter = RateLimiter::new();
        let config = make_config(10, Window::Second, None);
        limiter.try_consume("a", &config, "/test").unwrap();
        limiter.try_consume("b", &config, "/test").unwrap();
        limiter.try_consume("c", &config, "/test").unwrap();

        let active: HashSet<String> = ["a", "c"].iter().map(|s| (*s).into()).collect();
        limiter.purge_keys(&active);

        // a and c survive, b is gone.
        assert!(limiter.buckets.contains_key("a"));
        assert!(!limiter.buckets.contains_key("b"));
        assert!(limiter.buckets.contains_key("c"));
    }

    #[test]
    fn remove_key_deletes_single_bucket() {
        let limiter = RateLimiter::new();
        let config = make_config(10, Window::Second, None);
        limiter
            .try_consume("upstream:aaa", &config, "/test")
            .unwrap();
        limiter.try_consume("route:bbb", &config, "/test").unwrap();

        limiter.remove_key("upstream:aaa");

        assert!(!limiter.buckets.contains_key("upstream:aaa"));
        assert!(limiter.buckets.contains_key("route:bbb"));
    }

    #[test]
    fn remove_key_noop_for_missing_key() {
        let limiter = RateLimiter::new();
        // Should not panic.
        limiter.remove_key("nonexistent");
        assert!(limiter.buckets.is_empty());
    }

    #[test]
    fn purge_with_empty_set_removes_all() {
        let limiter = RateLimiter::new();
        let config = make_config(10, Window::Second, None);
        limiter.try_consume("x", &config, "/test").unwrap();
        limiter.try_consume("y", &config, "/test").unwrap();

        limiter.purge_keys(&HashSet::new());

        assert!(limiter.buckets.is_empty());
    }

    #[test]
    fn try_consume_returns_outcome_metadata() {
        let limiter = RateLimiter::new();
        let config = make_config(10, Window::Second, Some(10));

        let outcome = limiter.try_consume("test", &config, "/test").unwrap();
        assert_eq!(outcome.limit, 10);
        assert_eq!(outcome.remaining, 9);

        let now_epoch = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(outcome.reset_epoch >= now_epoch);
        assert!(outcome.reset_epoch <= now_epoch + 2);

        // Consume more and verify remaining decreases.
        let outcome2 = limiter.try_consume("test", &config, "/test").unwrap();
        assert_eq!(outcome2.remaining, 8);
    }

    #[test]
    fn error_includes_rate_limit_metadata() {
        let limiter = RateLimiter::new();
        let config = make_config(1, Window::Second, Some(1));
        limiter.try_consume("test", &config, "/test").unwrap();

        match limiter.try_consume("test", &config, "/test") {
            Err(DomainError::RateLimitExceeded {
                limit,
                remaining,
                reset_epoch,
                ..
            }) => {
                assert_eq!(limit, Some(1));
                assert_eq!(remaining, Some(0));
                let now_epoch = std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                assert!(reset_epoch.unwrap() >= now_epoch);
            }
            other => panic!("expected RateLimitExceeded, got {other:?}"),
        }
    }

    #[test]
    fn outcome_limit_falls_back_to_sustained_rate_without_burst() {
        let limiter = RateLimiter::new();
        let config = make_config(5, Window::Second, None); // no burst
        let outcome = limiter.try_consume("test", &config, "/test").unwrap();
        assert_eq!(
            outcome.limit, 5,
            "limit should equal sustained rate when burst is absent"
        );
    }

    #[test]
    fn outcome_reset_epoch_in_future() {
        let limiter = RateLimiter::new();
        let config = make_config(10, Window::Second, Some(10));
        // Consume 5 tokens so the bucket is partially drained.
        for _ in 0..5 {
            limiter.try_consume("test", &config, "/test").unwrap();
        }
        let outcome = limiter.try_consume("test", &config, "/test").unwrap();
        let now_epoch = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(
            outcome.reset_epoch >= now_epoch,
            "reset_epoch should be >= now"
        );
    }

    #[test]
    fn error_reset_epoch_is_time_until_full() {
        let limiter = RateLimiter::new();
        // capacity=5, rate=5/min → refill_rate ≈ 0.083 tok/s.
        // Consuming all 5 means secs_to_full ≈ 60s, but retry_after ≈ 12s (1 token).
        let config = make_config(5, Window::Minute, Some(5));
        for _ in 0..5 {
            limiter.try_consume("test", &config, "/test").unwrap();
        }

        match limiter.try_consume("test", &config, "/test") {
            Err(DomainError::RateLimitExceeded {
                retry_after_secs,
                reset_epoch,
                ..
            }) => {
                let now_epoch = std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let retry = retry_after_secs.unwrap();
                let reset = reset_epoch.unwrap();

                // reset_epoch should reflect time-until-full (≈60s), not retry_after (≈12s).
                assert!(
                    reset >= now_epoch + 50,
                    "reset_epoch ({reset}) should be ≈60s from now ({now_epoch}), \
                     not ≈retry_after ({retry}s)"
                );
                assert!(
                    retry < 20,
                    "retry_after ({retry}s) should be much less than secs_to_full"
                );
            }
            other => panic!("expected RateLimitExceeded, got {other:?}"),
        }
    }

    #[test]
    fn cost_greater_than_one_consumes_multiple_tokens() {
        let limiter = RateLimiter::new();
        let mut config = make_config(100, Window::Second, Some(10));
        config.cost = 3;

        // 10 tokens, cost=3: should allow 3 requests (9 tokens), fail on 4th (1 < 3).
        let o1 = limiter.try_consume("test", &config, "/test").unwrap();
        assert_eq!(o1.remaining, 7);

        let o2 = limiter.try_consume("test", &config, "/test").unwrap();
        assert_eq!(o2.remaining, 4);

        let o3 = limiter.try_consume("test", &config, "/test").unwrap();
        assert_eq!(o3.remaining, 1);

        let err = limiter.try_consume("test", &config, "/test").unwrap_err();
        assert!(matches!(err, DomainError::RateLimitExceeded { .. }));
    }

    #[test]
    fn error_omits_metadata_when_response_headers_false() {
        let limiter = RateLimiter::new();
        let mut config = make_config(1, Window::Minute, Some(1));
        config.response_headers = false;

        // Exhaust the bucket.
        limiter.try_consume("test", &config, "/test").unwrap();

        // Second request should be rejected.
        let err = limiter.try_consume("test", &config, "/test").unwrap_err();
        match err {
            DomainError::RateLimitExceeded {
                retry_after_secs,
                limit,
                remaining,
                reset_epoch,
                ..
            } => {
                assert!(
                    retry_after_secs.is_some(),
                    "retry_after_secs must always be present regardless of response_headers"
                );
                assert!(
                    limit.is_none(),
                    "limit must be None when response_headers is false"
                );
                assert!(
                    remaining.is_none(),
                    "remaining must be None when response_headers is false"
                );
                assert!(
                    reset_epoch.is_none(),
                    "reset_epoch must be None when response_headers is false"
                );
            }
            other => panic!("expected RateLimitExceeded, got {other:?}"),
        }
    }
}
