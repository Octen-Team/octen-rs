use std::time::Duration;

use crate::client::Client;
use crate::error::Error;
use crate::transport::Transport;

pub(crate) const DEFAULT_BASE_URL: &str = "https://api.octen.ai";
pub(crate) const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_MAX_RETRIES: u32 = 3;
const API_KEY_ENV: &str = "OCTEN_API_KEY";

/// Builds a [`Client`]; start with [`Client::builder`]. Problems are reported by [`ClientBuilder::build`].
#[derive(Debug, Default)]
pub struct ClientBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
    timeout: Option<Duration>,
    max_retries: Option<u32>,
    headers: Vec<(String, String)>,
    http_client: Option<reqwest::Client>,
    #[cfg(test)]
    pub(crate) hooks: crate::transport::Hooks,
}

impl ClientBuilder {
    /// The API key. A blank key falls back to the `OCTEN_API_KEY` environment variable.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// The API origin, `https://api.octen.ai` by default: an absolute http or https URL without
    /// credentials, query or fragment. Trailing slashes are dropped.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// The timeout of each attempt, including reading the response body. Once set it applies to
    /// every endpoint, Broad Search and Extract included, which otherwise get longer defaults.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// How many times a failed attempt is retried; 3 by default.
    pub fn max_retries(mut self, n: u32) -> Self {
        self.max_retries = Some(n);
        self
    }

    /// A header sent with every request. `x-api-key` is refused: use [`ClientBuilder::api_key`].
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Sends requests through this client, for proxies, custom TLS or connection pooling. Its own
    /// timeout applies on top of the SDK's, and its redirect policy is kept: the SDK's default client
    /// follows no redirects.
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Checks the configuration and builds the client. A missing API key, an invalid base URL,
    /// a zero timeout or an invalid header is an [`Error::Validation`].
    pub fn build(self) -> Result<Client, Error> {
        let mut key = self
            .api_key
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_owned();
        if key.is_empty() {
            key = std::env::var(API_KEY_ENV)
                .unwrap_or_default()
                .trim()
                .to_owned();
        }
        if key.is_empty() {
            return Err(Error::validation(
                "api_key",
                format!("An API key is required: pass it to the builder or set the {API_KEY_ENV} environment variable."),
            ));
        }
        if !is_field_value(&key) {
            return Err(Error::validation(
                "api_key",
                "The API key contains characters that are not valid in an HTTP header.",
            ));
        }
        let base_url = match &self.base_url {
            Some(url) => check_base_url(url)?,
            None => DEFAULT_BASE_URL.to_owned(),
        };
        if let Some(t) = self.timeout {
            check_timeout("timeout", t)?;
        }
        for (name, value) in &self.headers {
            check_header("header", name, value)?;
        }
        let http = match self.http_client {
            Some(c) => c,
            None => reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|e| Error::Connection {
                    source: Box::new(e),
                })?,
        };
        let transport = Transport::new(
            http,
            base_url,
            key,
            &self.headers,
            self.max_retries.unwrap_or(DEFAULT_MAX_RETRIES),
            #[cfg(test)]
            self.hooks,
        )?;
        Ok(Client::from_parts(transport, self.timeout))
    }
}

/// Options for one call, passed with `with_options` on a resource:
/// `client.embedding().with_options(opts).create(&params)`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct RequestOptions {
    /// The timeout of each attempt of this call, overriding the client's and the endpoint's default.
    pub timeout: Option<Duration>,
    /// Headers for this call; one with the name of a client header replaces it.
    pub headers: Vec<(String, String)>,
}

impl RequestOptions {
    /// Options that change nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the timeout of each attempt of this call.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Adds a header to this call.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub(crate) fn check(&self) -> Result<(), Error> {
        if let Some(t) = self.timeout {
            check_timeout("timeout", t)?;
        }
        for (name, value) in &self.headers {
            check_header("headers", name, value)?;
        }
        Ok(())
    }
}

pub(crate) fn check_timeout(field: &str, timeout: Duration) -> Result<(), Error> {
    if timeout.is_zero() {
        return Err(Error::validation(
            field,
            "The timeout must be greater than 0.",
        ));
    }
    Ok(())
}

fn check_base_url(raw: &str) -> Result<String, Error> {
    let bad = |msg: &str| Error::validation("base_url", msg);
    if raw.chars().any(char::is_whitespace) {
        return Err(bad("base_url must not contain whitespace."));
    }
    if raw.contains('?') {
        return Err(bad("base_url must not contain a query."));
    }
    if raw.contains('#') {
        return Err(bad("base_url must not contain a fragment."));
    }
    let url =
        reqwest::Url::parse(raw).map_err(|_| bad("base_url must be an absolute http(s) URL."))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none_or(str::is_empty) {
        return Err(bad("base_url must be an absolute http(s) URL."));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(bad(
            "base_url must not contain credentials (user:password@); pass the key with api_key.",
        ));
    }
    Ok(raw.trim_end_matches('/').to_owned())
}

pub(crate) fn check_header(field: &str, name: &str, value: &str) -> Result<(), Error> {
    if !is_token(name) {
        return Err(Error::validation(
            field,
            format!("Header name {name:?} is not a valid token."),
        ));
    }
    if name.eq_ignore_ascii_case("x-api-key") {
        return Err(Error::validation(
            field,
            "x-api-key cannot be set as a header; pass the key with api_key.",
        ));
    }
    if !is_field_value(value) {
        return Err(Error::validation(
            field,
            format!(
                "The value of header {name:?} must not contain control characters other than tab."
            ),
        ));
    }
    Ok(())
}

/// RFC 9110 forbids control bytes other than tab, including DEL, in a field value.
fn is_field_value(s: &str) -> bool {
    s.bytes().all(|c| (c >= 0x20 || c == b'\t') && c != 0x7f)
}

/// An RFC 9110 token, the grammar of a header name.
fn is_token(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&c))
}
