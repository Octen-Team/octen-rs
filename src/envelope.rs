use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use reqwest::header::HeaderMap;
use serde_json::value::RawValue;

use crate::error::ApiError;
use crate::retry::parse_retry_after;

const MESSAGE_RAW_LIMIT: usize = 500;
const MESSAGE_KEYS: [&str; 4] = ["msg", "message", "detail", "error"];
/// How many nested `error` objects are searched; each level is parsed once more.
const MAX_ERROR_DEPTH: usize = 8;

/// Top-level keys only: values stay unparsed slices of the body, so a large `data` is scanned, not built.
type TopLevel<'a> = HashMap<String, &'a RawValue>;

pub(crate) fn strip_bom(body: &[u8]) -> &[u8] {
    body.strip_prefix(b"\xef\xbb\xbf").unwrap_or(body)
}

/// Ok for a successful envelope: a 2xx whose body is a non-empty JSON object with no `code`, or a
/// `code` that is not a number, or a finite zero. Everything else is an [`ApiError`].
pub(crate) fn judge(status: u16, headers: &HeaderMap, body: &Bytes) -> Result<(), ApiError> {
    let text = strip_bom(body);
    let top = serde_json::from_slice::<TopLevel>(text).ok();
    if (200..300).contains(&status) {
        if let Some(obj) = &top {
            let failed = obj
                .get("code")
                .and_then(|raw| number(raw))
                .is_some_and(|f| !(f.is_finite() && f == 0.0));
            if !obj.is_empty() && !failed {
                return Ok(());
            }
        }
    }
    let mut message = top
        .as_ref()
        .and_then(|obj| find_message(obj, 0))
        .unwrap_or_default();
    if message.is_empty() {
        message = truncate_utf16(&String::from_utf8_lossy(text), MESSAGE_RAW_LIMIT);
    }
    if message.is_empty() {
        message = format!("HTTP {status}");
    }
    let retry_after = (status == 429)
        .then(|| header(headers, "retry-after"))
        .flatten()
        .and_then(|v| parse_retry_after(v, SystemTime::now()))
        .map(round_up_to_seconds);
    Err(ApiError {
        status,
        code: top
            .as_ref()
            .and_then(|obj| obj.get("code"))
            .and_then(|raw| number(raw))
            .filter(|f| f.is_finite()),
        message,
        request_id: request_id(headers, top.as_ref()),
        retry_after,
        body: body.clone(),
    })
}

/// The request ID of a response: the `x-request-id` header, else a `request_id` string in the body.
pub(crate) fn request_id(headers: &HeaderMap, top: Option<&TopLevel>) -> Option<String> {
    if let Some(v) = header(headers, "x-request-id").filter(|v| !v.is_empty()) {
        return Some(v.to_owned());
    }
    top?.get("request_id")
        .and_then(|raw| string(raw))
        .filter(|s| !s.is_empty())
}

pub(crate) fn request_id_of_body(headers: &HeaderMap, body: &[u8]) -> Option<String> {
    request_id(
        headers,
        serde_json::from_slice::<TopLevel>(strip_bom(body))
            .ok()
            .as_ref(),
    )
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|v| v.to_str().ok())
}

/// A JSON number as f64; one too large for f64 (1e999) is infinite rather than an error.
fn number(raw: &RawValue) -> Option<f64> {
    let s = raw.get();
    let first = s.as_bytes().first()?;
    if *first != b'-' && !first.is_ascii_digit() {
        return None;
    }
    s.parse::<f64>().ok()
}

fn string(raw: &RawValue) -> Option<String> {
    serde_json::from_str::<String>(raw.get()).ok()
}

/// msg, message, detail, error in order, recursing into an `error` object up to [`MAX_ERROR_DEPTH`].
fn find_message(obj: &TopLevel, depth: usize) -> Option<String> {
    for key in MESSAGE_KEYS {
        let Some(raw) = obj.get(key) else { continue };
        if let Some(s) = string(raw).filter(|s| !s.is_empty()) {
            return Some(s);
        }
        if key == "error" && depth < MAX_ERROR_DEPTH {
            let nested = serde_json::from_str::<TopLevel>(raw.get()).ok();
            if let Some(found) = nested.as_ref().and_then(|o| find_message(o, depth + 1)) {
                return Some(found);
            }
        }
    }
    None
}

/// The first `n` UTF-16 code units, like JavaScript's `String.slice`; a pair cut in half becomes U+FFFD.
fn truncate_utf16(s: &str, n: usize) -> String {
    let mut out = String::new();
    let mut units = 0;
    for c in s.chars() {
        let len = c.len_utf16();
        if units + len > n {
            if units < n {
                out.push('\u{fffd}');
            }
            break;
        }
        out.push(c);
        units += len;
    }
    out
}

/// The transport's message for a 3xx, which it never follows.
pub(crate) fn redirect_message(status: u16, headers: &HeaderMap) -> String {
    let mut msg = format!("HTTP {status} redirect");
    // Only the origin: a redirect target can be a signed or SSO URL carrying a token
    let origin = header(headers, "location")
        .and_then(|loc| reqwest::Url::parse(loc).ok())
        .map(|url| url.origin().ascii_serialization())
        .filter(|o| o != "null");
    if let Some(origin) = origin {
        msg.push_str(" to ");
        msg.push_str(&origin);
    }
    msg + "; redirects are not followed, check base_url"
}

fn round_up_to_seconds(d: Duration) -> Duration {
    if d.subsec_nanos() == 0 {
        return d;
    }
    d.as_secs()
        .checked_add(1)
        .map(Duration::from_secs)
        .unwrap_or(Duration::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderName, HeaderValue};
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::time::Duration;

    // The same cases the TypeScript and Go SDKs run: testdata/error_cases.json.
    #[derive(Deserialize)]
    struct SharedCase {
        name: String,
        status: u16,
        headers: HashMap<String, String>,
        body: String,
        expect: HashMap<String, serde_json::Value>,
    }

    fn class_of(e: &ApiError) -> &'static str {
        let matches = [
            (e.is_authentication(), "OctenAuthenticationError"),
            (e.is_permission_denied(), "OctenPermissionDeniedError"),
            (e.is_bad_request(), "OctenBadRequestError"),
            (e.is_rate_limited(), "OctenRateLimitError"),
            (e.is_server_error(), "OctenServerError"),
        ];
        let hits: Vec<_> = matches
            .iter()
            .filter(|(hit, _)| *hit)
            .map(|(_, c)| *c)
            .collect();
        assert!(hits.len() <= 1, "{hits:?} all match");
        hits.first().copied().unwrap_or("OctenAPIError")
    }

    #[test]
    fn shared_error_cases() {
        let cases: Vec<SharedCase> =
            serde_json::from_str(include_str!("../testdata/error_cases.json")).unwrap();
        assert_eq!(cases.len(), 48, "expected 48 shared cases");
        for c in cases {
            let mut headers = HeaderMap::new();
            for (k, v) in &c.headers {
                headers.insert(
                    HeaderName::from_bytes(k.as_bytes()).unwrap(),
                    HeaderValue::from_str(v).unwrap(),
                );
            }
            let body = Bytes::from(c.body.clone());
            let got = judge(c.status, &headers, &body);
            let ok = c.expect["ok"].as_bool().unwrap();
            if ok {
                assert!(got.is_ok(), "{}: expected success, got {:?}", c.name, got);
                continue;
            }
            let e = got.expect_err(&c.name);
            assert_eq!(e.status, c.status, "{}", c.name);
            assert_eq!(e.body, body, "{}: body must be the raw bytes", c.name);
            assert_eq!(
                class_of(&e),
                c.expect["class"].as_str().unwrap(),
                "{}",
                c.name
            );
            if let Some(code) = c.expect.get("code") {
                assert_eq!(e.code, code.as_f64(), "{}: code", c.name);
            }
            if let Some(message) = c.expect.get("message") {
                assert_eq!(e.message, message.as_str().unwrap(), "{}: message", c.name);
            }
            if let Some(id) = c.expect.get("requestId") {
                assert_eq!(
                    e.request_id.as_deref(),
                    id.as_str(),
                    "{}: request id",
                    c.name
                );
            }
            if let Some(after) = c.expect.get("retryAfter") {
                assert_eq!(
                    e.retry_after,
                    after.as_u64().map(Duration::from_secs),
                    "{}: retry after",
                    c.name
                );
            }
        }
    }

    // The previous full-decode rule (Go's TestJudgeResponseMatchesFullDecode) on body shapes the shared cases miss.
    #[test]
    fn success_rule_on_more_body_shapes() {
        let ok = |body: &str| judge(200, &HeaderMap::new(), &Bytes::from(body.to_owned())).is_ok();
        for body in [
            r#"{"code":0}"#,
            r#"{"code":0.0}"#,
            r#"{"code":-0}"#,
            r#"{"code":1e-999}"#,
            r#"{"code":"1"}"#,
            r#"{"code":null}"#,
            r#"{"code":true}"#,
            r#"{"code":{"n":1}}"#,
            r#"{"code":[1]}"#,
            r#"{"Code":1}"#,
            r#"{"code":2,"code":0}"#,
            r#"{"data":[1,2,3]}"#,
            "\u{feff}{\"code\":0}",
            " \n{\"a\":1}\n ",
        ] {
            assert!(ok(body), "{body:?} should succeed");
        }
        for body in [
            r#"{"code":1e999}"#,
            r#"{"code":-1e999}"#,
            r#"{"code":1}"#,
            r#"{"code":-2}"#,
            r#"{"code":0.5}"#,
            r#"{"code":0,"code":2}"#,
            "{}",
            "[]",
            r#"[{"code":0}]"#,
            "1",
            "\"x\"",
            "null",
            "",
            r#"{"a":1} x"#,
            r#"{"a":1"#,
            r#"{"code":01}"#,
        ] {
            assert!(!ok(body), "{body:?} should fail");
        }
    }

    #[test]
    fn redirect_is_not_success() {
        let mut h = HeaderMap::new();
        h.insert(
            "location",
            HeaderValue::from_static("https://example.com/next"),
        );
        let e = judge(301, &h, &Bytes::new()).unwrap_err();
        assert_eq!(e.status, 301);
    }

    // Each nested "error" object is parsed once more; a cap keeps a hostile body from exhausting the stack
    #[test]
    fn deeply_nested_error_objects_are_bounded() {
        let depth = 100_000;
        let body = r#"{"error":"#.repeat(depth) + r#"{"msg":"deep"}"# + &"}".repeat(depth);
        let started = std::time::Instant::now();
        let e = judge(500, &HeaderMap::new(), &Bytes::from(body)).unwrap_err();
        assert!(
            started.elapsed() < std::time::Duration::from_secs(2),
            "took {:?}",
            started.elapsed()
        );
        assert_eq!(e.status, 500);
        // A few levels are still searched
        let shallow = r#"{"error":{"error":{"message":"inner"}}}"#;
        let e = judge(500, &HeaderMap::new(), &Bytes::from(shallow)).unwrap_err();
        assert_eq!(e.message, "inner");
    }

    #[test]
    fn message_truncates_to_500_utf16_units() {
        let text = "é".repeat(300) + &"😀".repeat(200);
        let e = judge(500, &HeaderMap::new(), &Bytes::from(text.clone())).unwrap_err();
        // 300 one-unit chars, then 100 two-unit chars fill 500 units exactly
        assert_eq!(e.message, "é".repeat(300) + &"😀".repeat(100));
        let odd = "a".repeat(499) + "😀😀";
        let e = judge(500, &HeaderMap::new(), &Bytes::from(odd)).unwrap_err();
        // A pair cut in half leaves a lone surrogate, which a Rust string holds as U+FFFD
        assert_eq!(e.message, "a".repeat(499) + "\u{fffd}");
        let e = judge(502, &HeaderMap::new(), &Bytes::new()).unwrap_err();
        assert_eq!(e.message, "HTTP 502");
    }
}
