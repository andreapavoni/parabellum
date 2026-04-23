use std::sync::atomic::{AtomicU64, Ordering};

static AUTH_SUCCESS: AtomicU64 = AtomicU64::new(0);
static AUTH_FAILURE: AtomicU64 = AtomicU64::new(0);
static REFRESH_SUCCESS: AtomicU64 = AtomicU64::new(0);
static REFRESH_FAILURE: AtomicU64 = AtomicU64::new(0);
static TOKEN_EXPIRED: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct AuthMetricsSnapshot {
    pub auth_success: u64,
    pub auth_failure: u64,
    pub refresh_success: u64,
    pub refresh_failure: u64,
    pub token_expired: u64,
}

pub fn inc_auth_success() {
    AUTH_SUCCESS.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_auth_failure() {
    AUTH_FAILURE.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_refresh_success() {
    REFRESH_SUCCESS.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_refresh_failure() {
    REFRESH_FAILURE.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_token_expired() {
    TOKEN_EXPIRED.fetch_add(1, Ordering::Relaxed);
}

pub fn snapshot() -> AuthMetricsSnapshot {
    AuthMetricsSnapshot {
        auth_success: AUTH_SUCCESS.load(Ordering::Relaxed),
        auth_failure: AUTH_FAILURE.load(Ordering::Relaxed),
        refresh_success: REFRESH_SUCCESS.load(Ordering::Relaxed),
        refresh_failure: REFRESH_FAILURE.load(Ordering::Relaxed),
        token_expired: TOKEN_EXPIRED.load(Ordering::Relaxed),
    }
}
