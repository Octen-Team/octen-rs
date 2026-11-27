use std::time::{Duration, SystemTime};

pub(crate) const MAX_RETRY_DELAY: Duration = Duration::from_secs(60);
const BASE_RETRY_DELAY: Duration = Duration::from_secs(1);

pub(crate) fn is_retryable_status(status: u16) -> bool {
    matches!(status, 408 | 429 | 500 | 502 | 503 | 504)
}

/// Non-negative integer seconds or an HTTP date (a past date yields zero); anything else is ignored.
/// Seconds too large to represent saturate, which still means "wait too long".
pub(crate) fn parse_retry_after(value: &str, now: SystemTime) -> Option<Duration> {
    let v = value.trim();
    if v.is_empty() {
        return None;
    }
    if v.bytes().all(|b| b.is_ascii_digit()) {
        return Some(
            v.parse::<u64>()
                .map(Duration::from_secs)
                .unwrap_or(Duration::MAX),
        );
    }
    let at = httpdate::parse_http_date(v).ok()?;
    Some(at.duration_since(now).unwrap_or(Duration::ZERO))
}

/// The wait before the next attempt, or `None` to give up. `attempt` starts at 1; `status` is
/// `None` for a network error. Retry-After is honoured only for 429 and 503, up to 60 s.
pub(crate) fn retry_delay(
    attempt: u32,
    status: Option<u16>,
    retry_after: Option<Duration>,
    jitter: f64,
) -> Option<Duration> {
    if let (Some(429 | 503), Some(wait)) = (status, retry_after) {
        return (wait <= MAX_RETRY_DELAY).then_some(wait);
    }
    let exp = attempt.max(1) - 1;
    // Float math keeps large attempt numbers from overflowing before the cap applies.
    let backoff =
        BASE_RETRY_DELAY.as_secs_f64() * 2f64.powi(exp.min(64) as i32) * (1.0 + 0.25 * jitter);
    if backoff >= MAX_RETRY_DELAY.as_secs_f64() {
        return Some(MAX_RETRY_DELAY);
    }
    Some(Duration::from_secs_f64(backoff))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_statuses() {
        for s in [408, 429, 500, 502, 503, 504] {
            assert!(is_retryable_status(s), "{s}");
        }
        for s in [200, 400, 401, 403, 404, 409, 422, 501, 505] {
            assert!(!is_retryable_status(s), "{s}");
        }
    }

    #[test]
    fn backoff_doubles_with_jitter_and_caps_at_60s() {
        assert_eq!(
            retry_delay(1, Some(500), None, 0.0),
            Some(Duration::from_secs(1))
        );
        assert_eq!(
            retry_delay(2, None, None, 0.0),
            Some(Duration::from_secs(2))
        );
        assert_eq!(
            retry_delay(3, Some(502), None, 1.0),
            Some(Duration::from_secs(5))
        );
        assert_eq!(retry_delay(7, Some(500), None, 0.0), Some(MAX_RETRY_DELAY));
        assert_eq!(
            retry_delay(u32::MAX, None, None, 1.0),
            Some(MAX_RETRY_DELAY)
        );
    }

    #[test]
    fn retry_after_only_for_429_and_503() {
        let five = Some(Duration::from_secs(5));
        assert_eq!(retry_delay(1, Some(429), five, 0.0), five);
        assert_eq!(retry_delay(1, Some(503), five, 0.0), five);
        // Other statuses fall back to the backoff
        assert_eq!(
            retry_delay(1, Some(500), five, 0.0),
            Some(Duration::from_secs(1))
        );
        assert_eq!(
            retry_delay(1, Some(429), Some(MAX_RETRY_DELAY), 0.0),
            Some(MAX_RETRY_DELAY)
        );
        assert_eq!(
            retry_delay(1, Some(429), Some(Duration::from_secs(61)), 0.0),
            None
        );
    }

    #[test]
    fn parses_retry_after_seconds_and_dates() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        assert_eq!(parse_retry_after(" 7 ", now), Some(Duration::from_secs(7)));
        assert_eq!(parse_retry_after("0", now), Some(Duration::ZERO));
        assert_eq!(
            parse_retry_after("99999999999999999999999", now),
            Some(Duration::MAX)
        );
        for bad in ["", "soon", "-1", "1.5"] {
            assert_eq!(parse_retry_after(bad, now), None, "{bad:?}");
        }
        let later = httpdate::fmt_http_date(now + Duration::from_secs(30));
        assert_eq!(
            parse_retry_after(&later, now),
            Some(Duration::from_secs(30))
        );
        let earlier = httpdate::fmt_http_date(now - Duration::from_secs(30));
        assert_eq!(parse_retry_after(&earlier, now), Some(Duration::ZERO));
        // RFC 850 and asctime, which HTTP dates also allow
        let base = SystemTime::UNIX_EPOCH + Duration::from_secs(784_111_777);
        for date in ["Sunday, 06-Nov-94 08:49:37 GMT", "Sun Nov  6 08:49:37 1994"] {
            assert_eq!(
                parse_retry_after(date, base),
                Some(Duration::ZERO),
                "{date}"
            );
        }
    }
}
