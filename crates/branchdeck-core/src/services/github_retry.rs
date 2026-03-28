use crate::error::AppError;
use log::{debug, warn};
use std::future::Future;
use std::time::Duration;

/// Maximum number of retry attempts for transient GitHub API errors.
const MAX_RETRIES: u32 = 3;

/// Base delay between retries (30s per NFR12).
const BASE_DELAY_SECS: u64 = 30;

/// Maximum delay cap (5 minutes).
const MAX_DELAY_SECS: u64 = 300;

/// Classify whether an `AppError` is transient (worth retrying).
///
/// Retryable: rate limit (429), server errors (5xx), connection errors.
/// NOT retryable: auth errors (401), not found (404), other client errors.
#[must_use]
pub fn is_transient(err: &AppError) -> bool {
    let msg = err.to_string();
    let lower = msg.to_lowercase();

    // Rate limit — match specific HTTP status patterns to avoid false positives
    // from unrelated numbers (e.g., port 50300, PR #5030)
    if lower.contains("rate limit")
        || crate::util::contains_http_status(&lower, "429")
        || crate::util::contains_http_status(&lower, "500")
        || crate::util::contains_http_status(&lower, "502")
        || crate::util::contains_http_status(&lower, "503")
        || crate::util::contains_http_status(&lower, "504")
    {
        return true;
    }

    // Connection errors — these are full phrases, low false-positive risk
    if lower.contains("connection refused")
        || lower.contains("timed out")
        || lower.contains("timeout")
        || lower.contains("connect error")
        || lower.contains("dns error")
        || lower.contains("connection reset")
    {
        return true;
    }

    false
}

/// Compute the backoff delay for a given attempt (0-indexed).
/// Uses exponential backoff: `base * 2^attempt`, capped at `MAX_DELAY_SECS`.
#[must_use]
pub fn retry_delay(attempt: u32) -> Duration {
    let shift = attempt.min(63);
    let delay_secs = BASE_DELAY_SECS.saturating_mul(1u64 << shift);
    Duration::from_secs(delay_secs.min(MAX_DELAY_SECS))
}

/// Execute an async operation with retry on transient errors.
///
/// Retries up to `MAX_RETRIES` times with exponential backoff (30s base per NFR12).
/// Does NOT retry on permanent errors (401, 404, other client errors).
///
/// # Errors
///
/// Returns the last error if all retries are exhausted or a non-transient error occurs.
pub async fn with_retry<F, Fut, T>(label: &str, f: F) -> Result<T, AppError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    let mut last_err: Option<AppError> = None;

    for attempt in 0..=MAX_RETRIES {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                if !is_transient(&e) {
                    debug!("GitHub {label}: non-transient error, not retrying: {e}");
                    return Err(e);
                }

                last_err = Some(e);

                if attempt < MAX_RETRIES {
                    let delay = retry_delay(attempt);
                    warn!(
                        "GitHub {label}: transient error (attempt {}/{}), retrying in {}s",
                        attempt + 1,
                        MAX_RETRIES,
                        delay.as_secs()
                    );
                    tokio::time::sleep(delay).await;
                } else {
                    warn!("GitHub {label}: transient error, all {MAX_RETRIES} retries exhausted");
                }
            }
        }
    }

    Err(last_err.unwrap_or_else(|| AppError::GitHub(format!("{label}: retries exhausted"))))
}

/// Invalidate the PR cache. Called on network errors to prevent serving stale data.
pub fn invalidate_pr_cache() {
    if let Some(cache) = super::github::pr_cache() {
        if let Ok(mut guard) = cache.lock() {
            let count = guard.len();
            guard.clear();
            if count > 0 {
                debug!("Invalidated {count} PR cache entries after network error");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn test_is_transient_rate_limit() {
        let err = AppError::GitHub("rate limit exceeded (429)".to_string());
        assert!(is_transient(&err));
    }

    #[test]
    fn test_is_transient_server_error() {
        let err = AppError::GitHub("Internal Server Error 500".to_string());
        assert!(is_transient(&err));
    }

    #[test]
    fn test_is_transient_502() {
        let err = AppError::GitHub("502 Bad Gateway".to_string());
        assert!(is_transient(&err));
    }

    #[test]
    fn test_not_transient_port_number() {
        let err = AppError::GitHub("connect to port 50300 failed".to_string());
        assert!(
            !is_transient(&err),
            "port 50300 should not match status 503"
        );
    }

    #[test]
    fn test_not_transient_pr_number() {
        let err = AppError::GitHub("PR #5030 not found".to_string());
        assert!(!is_transient(&err), "PR #5030 should not match status 503");
    }

    #[test]
    fn test_is_transient_connection_refused() {
        let err = AppError::GitHub("connection refused".to_string());
        assert!(is_transient(&err));
    }

    #[test]
    fn test_is_transient_timeout() {
        let err = AppError::GitHub("request timed out".to_string());
        assert!(is_transient(&err));
    }

    #[test]
    fn test_not_transient_auth() {
        let err = AppError::GitHub("Bad credentials (401)".to_string());
        assert!(!is_transient(&err));
    }

    #[test]
    fn test_not_transient_not_found() {
        let err = AppError::GitHub("Not Found".to_string());
        assert!(!is_transient(&err));
    }

    #[test]
    fn test_retry_delay_exponential() {
        assert_eq!(retry_delay(0), Duration::from_secs(30));
        assert_eq!(retry_delay(1), Duration::from_secs(60));
        assert_eq!(retry_delay(2), Duration::from_secs(120));
    }

    #[test]
    fn test_retry_delay_capped() {
        assert_eq!(retry_delay(10), Duration::from_secs(MAX_DELAY_SECS));
    }

    #[tokio::test]
    async fn test_with_retry_success_first_try() {
        let result = with_retry("test", || async { Ok::<_, AppError>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_retry_permanent_error_no_retry() {
        let attempt = std::sync::atomic::AtomicU32::new(0);
        let result = with_retry("test", || {
            attempt.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            async { Err::<i32, _>(AppError::GitHub("Not Found".to_string())) }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(attempt.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
}
