use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use std::sync::{
    Arc,
    atomic::{AtomicI64, AtomicU32, Ordering},
};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

#[derive(Clone)]
pub struct AuthBackoffState {
    failure_count: Arc<AtomicU32>,
    last_attempt: Arc<AtomicI64>,
    reset_window_seconds: i64,
}

impl AuthBackoffState {
    pub fn new(reset_window_seconds: i64) -> Self {
        Self {
            failure_count: Arc::new(AtomicU32::new(0)),
            last_attempt: Arc::new(AtomicI64::new(0)),
            reset_window_seconds,
        }
    }

    pub fn should_reset(&self) -> bool {
        let now = Utc::now().timestamp();
        let last = self.last_attempt.load(Ordering::SeqCst);
        now - last > self.reset_window_seconds
    }

    pub fn record_attempt(&self) {
        let now = Utc::now().timestamp();
        self.last_attempt.store(now, Ordering::SeqCst);
    }

    pub fn increment_failures(&self) -> u32 {
        self.failure_count.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn reset_failures(&self) {
        self.failure_count.store(0, Ordering::SeqCst);
        info!("Authentication backoff reset");
    }

    pub fn get_failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::SeqCst)
    }

    pub fn calculate_delay(&self, failures: u32) -> Duration {
        if failures == 0 {
            return Duration::from_secs(0);
        }

        // Exponential backoff: 2^(failures-1) seconds, max 30 seconds
        // (OTP codes last for 30 seconds, we allow a skew of 1 code, any
        // longer than this would prevent login altogether)
        let delay_seconds = std::cmp::min(2_u64.pow(failures.saturating_sub(1)), 30);

        Duration::from_secs(delay_seconds)
    }
}

pub async fn auth_backoff_middleware(
    State(backoff_state): State<AuthBackoffState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Auto-reset if enough time has passed
    if backoff_state.should_reset() && backoff_state.get_failure_count() > 0 {
        backoff_state.reset_failures();
    }

    let current_failures = backoff_state.get_failure_count();

    // Apply exponential backoff
    if current_failures > 0 {
        let delay = backoff_state.calculate_delay(current_failures);

        warn!(
            "Auth backoff: delaying {:?} due to {} failures",
            delay, current_failures
        );

        sleep(delay).await;
    }

    backoff_state.record_attempt();
    request.extensions_mut().insert(backoff_state.clone());

    let response = next.run(request).await;

    // Handle response
    match response.status() {
        StatusCode::UNAUTHORIZED => {
            let new_count = backoff_state.increment_failures();
            warn!("Auth failed. Failure count: {}", new_count);
        }
        status if status.is_success() => {
            backoff_state.reset_failures();
        }
        _ => {} // Other errors don't affect backoff
    }

    response
}
