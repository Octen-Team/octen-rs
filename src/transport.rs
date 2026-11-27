use std::time::{Duration, SystemTime};

use bytes::Bytes;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, CONTENT_TYPE, USER_AGENT};

use crate::envelope::{judge, redirect_message};
use crate::error::Error;
use crate::retry::{is_retryable_status, parse_retry_after, retry_delay};

pub(crate) const USER_AGENT_VALUE: &str = concat!("octen-rust-sdk/", env!("CARGO_PKG_VERSION"));

/// Test hooks: record backoff waits instead of sleeping, and fix the jitter.
#[cfg(test)]
#[derive(Clone, Default)]
pub(crate) struct Hooks {
    pub(crate) waits: Option<std::sync::Arc<std::sync::Mutex<Vec<Duration>>>>,
    pub(crate) jitter: Option<f64>,
    /// The attempt timeout of each call, recorded when set.
    pub(crate) timeouts: Option<std::sync::Arc<std::sync::Mutex<Vec<Duration>>>>,
}

#[cfg(test)]
impl std::fmt::Debug for Hooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Hooks")
    }
}

pub(crate) struct Transport {
    http: reqwest::Client,
    base_url: String,
    headers: HeaderMap,
    max_retries: u32,
    #[cfg(test)]
    hooks: Hooks,
}

// The HTTP client is left out: its default headers, set on a custom builder, can hold a credential.
impl std::fmt::Debug for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transport")
            .field("base_url", &self.base_url)
            .field("headers", &self.headers.keys().collect::<Vec<_>>())
            .field("max_retries", &self.max_retries)
            .finish_non_exhaustive()
    }
}

/// A response whose envelope was judged successful.
pub(crate) struct Success {
    pub(crate) headers: HeaderMap,
    pub(crate) body: Bytes,
}

enum Attempt {
    Response {
        status: u16,
        headers: HeaderMap,
        body: Bytes,
    },
    Timeout,
    Network {
        source: reqwest::Error,
    },
}

impl Transport {
    pub(crate) fn new(
        http: reqwest::Client,
        base_url: String,
        api_key: String,
        extra: &[(String, String)],
        max_retries: u32,
        #[cfg(test)] hooks: Hooks,
    ) -> Result<Self, Error> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));
        merge_headers(&mut headers, extra, "header")?;
        // Written last so no default or extra header can replace the key.
        let mut key = HeaderValue::from_str(&api_key).map_err(|_| {
            Error::validation(
                "api_key",
                "The API key contains characters that are not valid in an HTTP header.",
            )
        })?;
        key.set_sensitive(true);
        headers.insert("x-api-key", key);
        Ok(Self {
            http,
            base_url,
            headers,
            max_retries,
            #[cfg(test)]
            hooks,
        })
    }

    /// Posts `body` as JSON, retrying as the retry rules allow, and returns a successful envelope.
    /// The body bytes are shared, so every attempt sends exactly the same request.
    pub(crate) async fn post(
        &self,
        path: &str,
        body: Bytes,
        timeout: Duration,
        extra: &[(String, String)],
    ) -> Result<Success, Error> {
        #[cfg(test)]
        if let Some(t) = &self.hooks.timeouts {
            t.lock().unwrap().push(timeout);
        }
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let mut headers = self.headers.clone();
        merge_headers(&mut headers, extra, "headers")?;
        let key = self.headers.get("x-api-key").cloned();
        if let Some(key) = key {
            headers.insert("x-api-key", key);
        }

        let mut attempt: u32 = 1;
        loop {
            let (err, status, retry_after, retryable) =
                match self.attempt(&url, &headers, body.clone(), timeout).await {
                    Attempt::Response {
                        status,
                        headers,
                        body,
                    } => match judge(status, &headers, &body) {
                        Ok(()) => return Ok(Success { headers, body }),
                        Err(mut e) => {
                            if (300..400).contains(&status) {
                                e.message = redirect_message(status, &headers);
                                return Err(Error::Api(e));
                            }
                            let after = headers
                                .get("retry-after")
                                .and_then(|v| v.to_str().ok())
                                .and_then(|v| parse_retry_after(v, SystemTime::now()));
                            (
                                Error::Api(e),
                                Some(status),
                                after,
                                is_retryable_status(status),
                            )
                        }
                    },
                    Attempt::Timeout => return Err(Error::Timeout { timeout }),
                    // Only a failure to connect is known not to have reached the server.
                    Attempt::Network { source } => {
                        // A timeout of the HTTP client itself, even while connecting, is final like the SDK's own.
                        let pre_send =
                            retryable_network_error(source.is_connect(), source.is_timeout());
                        (
                            Error::Connection {
                                source: Box::new(source),
                            },
                            None,
                            None,
                            pre_send,
                        )
                    }
                };
            if !retryable || attempt > self.max_retries {
                return Err(err);
            }
            let Some(delay) = retry_delay(attempt, status, retry_after, self.jitter()) else {
                return Err(err);
            };
            self.wait(delay).await;
            attempt += 1;
        }
    }

    /// One request; the timeout covers sending it and reading the whole body.
    async fn attempt(
        &self,
        url: &str,
        headers: &HeaderMap,
        body: Bytes,
        timeout: Duration,
    ) -> Attempt {
        let send = async {
            let resp = self
                .http
                .post(url)
                .headers(headers.clone())
                .body(body)
                .send()
                .await?;
            let status = resp.status().as_u16();
            let headers = resp.headers().clone();
            let body = resp.bytes().await?;
            Ok::<_, reqwest::Error>((status, headers, body))
        };
        match tokio::time::timeout(timeout, send).await {
            Err(_) => Attempt::Timeout,
            Ok(Err(source)) => Attempt::Network { source },
            Ok(Ok((status, headers, body))) => Attempt::Response {
                status,
                headers,
                body,
            },
        }
    }

    fn jitter(&self) -> f64 {
        #[cfg(test)]
        if let Some(j) = self.hooks.jitter {
            return j;
        }
        fastrand::f64()
    }

    async fn wait(&self, delay: Duration) {
        #[cfg(test)]
        if let Some(waits) = &self.hooks.waits {
            waits.lock().unwrap().push(delay);
            return;
        }
        tokio::time::sleep(delay).await;
    }
}

/// Only a failure while connecting never reached the server; a timeout, even while connecting, is final.
fn retryable_network_error(is_connect: bool, is_timeout: bool) -> bool {
    is_connect && !is_timeout
}

/// Adds `extra` to `headers`, each replacing any header of the same name.
fn merge_headers(
    headers: &mut HeaderMap,
    extra: &[(String, String)],
    field: &str,
) -> Result<(), Error> {
    for (name, value) in extra {
        crate::config::check_header(field, name, value)?;
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| {
            Error::validation(field, format!("Header name {name:?} is not a valid token."))
        })?;
        let mut value = HeaderValue::from_str(value).map_err(|_| {
            Error::validation(field, format!("The value of header {name:?} is not valid."))
        })?;
        // Shown as "Sensitive" in Debug output: a custom header can carry a credential.
        value.set_sensitive(true);
        headers.insert(name, value);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::retryable_network_error;

    #[test]
    fn only_connect_failures_that_are_not_timeouts_are_retried() {
        assert!(retryable_network_error(true, false));
        assert!(
            !retryable_network_error(true, true),
            "a connect timeout must not be retried"
        );
        assert!(!retryable_network_error(false, true));
        assert!(!retryable_network_error(false, false));
    }
}
