use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tracing::{warn, debug};

/// Error classification for connector I/O operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorClassification {
    /// Temporary failures suitable for exponential backoff retries (e.g., HTTP 429, 500, 502, 503, 504, timeout)
    Transient,
    /// Permanent failures that will not succeed upon retry (e.g., HTTP 400, 401, 403, 404, 422)
    Permanent,
}

impl ErrorClassification {
    pub fn from_http_status(status_code: u16) -> Self {
        match status_code {
            429 | 500 | 502 | 503 | 504 => ErrorClassification::Transient,
            _ => ErrorClassification::Permanent,
        }
    }
}

/// Unified Connector Error Taxonomy
#[derive(Debug, thiserror::Error, Clone)]
pub enum ConnectorError {
    #[error("Transient error (retryable): {message}")]
    Transient { message: String, retry_after_secs: Option<u64> },

    #[error("Permanent failure: {message}")]
    Permanent { message: String },

    #[error("Rate limit exceeded: retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("Circuit breaker is OPEN for connector '{connector_id}'")]
    CircuitOpen { connector_id: String },

    #[error("Retry budget exhausted for connector '{connector_id}'")]
    BudgetExhausted { connector_id: String },

    #[error("Operation timed out after {timeout_secs}s")]
    Timeout { timeout_secs: u64 },
}

/// Configurable Exponential Backoff Retry Policy with Full Jitter
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub backoff_factor: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_backoff_ms: 500,
            max_backoff_ms: 60_000,
            backoff_factor: 2.0,
        }
    }
}

impl RetryPolicy {
    /// Calculate backoff duration with full jitter or header override
    pub fn calculate_backoff(&self, attempt: u32, retry_after_secs: Option<u64>) -> Duration {
        if let Some(secs) = retry_after_secs {
            return Duration::from_secs(secs);
        }

        let exponential = (self.initial_backoff_ms as f64) * self.backoff_factor.powi(attempt as i32);
        let capped = exponential.min(self.max_backoff_ms as f64) as u64;

        // Simple pseudo-random full jitter based on system time nanos
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(12345);
        let jitter = (nanos as u64 % capped.max(1)) + 1;

        Duration::from_millis(jitter)
    }
}

/// Token Bucket Retry Budget to prevent retry storms
#[derive(Debug)]
pub struct RetryBudget {
    max_tokens: u32,
    available_tokens: AtomicU32,
    deposit_cost: u32,
    withdraw_cost: u32,
}

impl RetryBudget {
    pub fn new(max_tokens: u32) -> Self {
        Self {
            max_tokens,
            available_tokens: AtomicU32::new(max_tokens),
            deposit_cost: 1,
            withdraw_cost: 10,
        }
    }

    /// Check if enough budget is available to perform a retry attempt
    pub fn can_retry(&self) -> bool {
        let current = self.available_tokens.load(Ordering::Relaxed);
        current >= self.withdraw_cost
    }

    /// Record a successful call, replenishing retry tokens
    pub fn record_success(&self) {
        let _ = self.available_tokens.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |val| {
            Some((val + self.deposit_cost).min(self.max_tokens))
        });
    }

    /// Consume tokens for a retry attempt. Returns true if granted.
    pub fn consume_retry(&self) -> bool {
        self.available_tokens
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |val| {
                if val >= self.withdraw_cost {
                    Some(val - self.withdraw_cost)
                } else {
                    None
                }
            })
            .is_ok()
    }

    pub fn available_tokens(&self) -> u32 {
        self.available_tokens.load(Ordering::Relaxed)
    }
}

impl Default for RetryBudget {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Circuit Breaker States: Closed (0), Open (1), HalfOpen (2)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed = 0,
    Open = 1,
    HalfOpen = 2,
}

/// Thread-safe Circuit Breaker State Machine
#[derive(Debug)]
pub struct CircuitBreaker {
    connector_id: String,
    state: AtomicU8,
    failure_count: AtomicU32,
    failure_threshold: u32,
    reset_timeout_secs: u64,
    last_state_change: std::sync::Mutex<Instant>,
}

impl CircuitBreaker {
    pub fn new(connector_id: impl Into<String>, failure_threshold: u32, reset_timeout_secs: u64) -> Self {
        Self {
            connector_id: connector_id.into(),
            state: AtomicU8::new(CircuitState::Closed as u8),
            failure_count: AtomicU32::new(0),
            failure_threshold,
            reset_timeout_secs,
            last_state_change: std::sync::Mutex::new(Instant::now()),
        }
    }

    pub fn state(&self) -> CircuitState {
        let current = self.state.load(Ordering::Relaxed);
        match current {
            1 => {
                // Check if reset timeout elapsed to transition to HalfOpen
                let last = *self.last_state_change.lock().unwrap();
                if last.elapsed() >= Duration::from_secs(self.reset_timeout_secs) {
                    self.state.store(CircuitState::HalfOpen as u8, Ordering::Relaxed);
                    CircuitState::HalfOpen
                } else {
                    CircuitState::Open
                }
            }
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed,
        }
    }

    pub fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        if self.state.load(Ordering::Relaxed) != CircuitState::Closed as u8 {
            self.state.store(CircuitState::Closed as u8, Ordering::Relaxed);
            *self.last_state_change.lock().unwrap() = Instant::now();
            debug!("Circuit Breaker for '{}' reset to CLOSED", self.connector_id);
        }
    }

    pub fn record_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        if count >= self.failure_threshold {
            self.state.store(CircuitState::Open as u8, Ordering::Relaxed);
            *self.last_state_change.lock().unwrap() = Instant::now();
            warn!("Circuit Breaker for '{}' tripped to OPEN (failures: {})", self.connector_id, count);
        }
    }
}

/// Unified Resilience Manager for a Connector
#[derive(Debug)]
pub struct ResilienceManager {
    pub connector_id: String,
    pub retry_policy: RetryPolicy,
    pub retry_budget: Arc<RetryBudget>,
    pub circuit_breaker: Arc<CircuitBreaker>,
    pub bulkhead: Arc<Semaphore>,
}

impl ResilienceManager {
    pub fn new(connector_id: impl Into<String>, max_concurrent: usize) -> Self {
        let cid = connector_id.into();
        Self {
            connector_id: cid.clone(),
            retry_policy: RetryPolicy::default(),
            retry_budget: Arc::new(RetryBudget::default()),
            circuit_breaker: Arc::new(CircuitBreaker::new(&cid, 5, 30)),
            bulkhead: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// Execute an operation wrapped with Bulkhead isolation, Circuit Breaker checks, Retry Budget, and Backoff
    pub async fn execute<F, Fut, T>(&self, mut f: F) -> Result<T, ConnectorError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, ConnectorError>>,
    {
        // 1. Check Circuit Breaker
        if self.circuit_breaker.state() == CircuitState::Open {
            return Err(ConnectorError::CircuitOpen {
                connector_id: self.connector_id.clone(),
            });
        }

        // 2. Acquire Bulkhead Permit
        let _permit = self
            .bulkhead
            .acquire()
            .await
            .map_err(|_| ConnectorError::Permanent {
                message: "Bulkhead semaphore closed".to_string(),
            })?;

        let mut attempt = 0;
        loop {
            match f().await {
                Ok(val) => {
                    self.retry_budget.record_success();
                    self.circuit_breaker.record_success();
                    return Ok(val);
                }
                Err(ConnectorError::Permanent { message }) => {
                    self.circuit_breaker.record_failure();
                    return Err(ConnectorError::Permanent { message });
                }
                Err(err) => {
                    self.circuit_breaker.record_failure();
                    attempt += 1;

                    if attempt > self.retry_policy.max_retries {
                        return Err(err);
                    }

                    if !self.retry_budget.consume_retry() {
                        return Err(ConnectorError::BudgetExhausted {
                            connector_id: self.connector_id.clone(),
                        });
                    }

                    let retry_after = match &err {
                        ConnectorError::Transient { retry_after_secs, .. } => *retry_after_secs,
                        ConnectorError::RateLimited { retry_after_secs } => Some(*retry_after_secs),
                        _ => None,
                    };

                    let delay = self.retry_policy.calculate_backoff(attempt, retry_after);
                    warn!(
                        "Retrying connector '{}' (attempt {}/{}) after {:?}",
                        self.connector_id, attempt, self.retry_policy.max_retries, delay
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
}
