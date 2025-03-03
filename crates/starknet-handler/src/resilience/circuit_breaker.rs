use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    state: Arc<CircuitBreakerState>,
    config: CircuitBreakerConfig,
}

#[derive(Debug)]
struct CircuitBreakerState {
    is_open: AtomicBool,
    failure_count: AtomicU64,
    last_failure_time: Mutex<Option<Instant>>,
    half_open_allowed: AtomicBool,
}

#[derive(Debug, Clone, Copy)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u64,
    pub reset_timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(60),
        }
    }
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(CircuitBreakerState {
                is_open: AtomicBool::new(false),
                failure_count: AtomicU64::new(0),
                last_failure_time: Mutex::new(None),
                half_open_allowed: AtomicBool::new(false),
            }),
            config,
        }
    }

    pub fn is_closed(&self) -> bool {
        !self.state.is_open.load(Ordering::Relaxed)
    }

    pub async fn allow_request(&self) -> bool {
        if !self.state.is_open.load(Ordering::Relaxed) {
            return true;
        }

        // Circuit is open, check if we should try half-open state
        let last_failure = self.state.last_failure_time.lock().await;

        if let Some(time) = *last_failure {
            if time.elapsed() >= self.config.reset_timeout {
                // Allow one request to test the service
                if self
                    .state
                    .half_open_allowed
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    tracing::info!("Circuit breaker entering half-open state");
                    return true;
                }
            }
        }

        false
    }

    pub async fn on_success(&self) {
        if self.state.is_open.load(Ordering::Relaxed) {
            tracing::info!("Circuit breaker closing after successful request");
            self.state.is_open.store(false, Ordering::Relaxed);
            self.state.failure_count.store(0, Ordering::Relaxed);
            self.state.half_open_allowed.store(false, Ordering::Relaxed);
        }
    }

    pub async fn on_failure(&self) {
        let current_count = self.state.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        let mut last_failure = self.state.last_failure_time.lock().await;
        *last_failure = Some(Instant::now());

        if current_count >= self.config.failure_threshold
            && !self.state.is_open.load(Ordering::Relaxed)
        {
            tracing::warn!(
                "Circuit breaker opening after {} consecutive failures",
                current_count
            );
            self.state.is_open.store(true, Ordering::Relaxed);
            self.state.half_open_allowed.store(false, Ordering::Relaxed);
        } else if self.state.half_open_allowed.load(Ordering::Relaxed) {
            tracing::warn!("Circuit breaker remaining open after test request failure");
            self.state.half_open_allowed.store(false, Ordering::Relaxed);
        }
    }

    pub async fn reset(&self) {
        tracing::info!("Circuit breaker manually reset");
        self.state.is_open.store(false, Ordering::Relaxed);
        self.state.failure_count.store(0, Ordering::Relaxed);
        self.state.half_open_allowed.store(false, Ordering::Relaxed);
        let mut last_failure = self.state.last_failure_time.lock().await;
        *last_failure = None;
    }
}
