use std::sync::Arc;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::config::{ClientBuilder, RequestOptions};
use crate::envelope::{request_id_of_body, strip_bom};
use crate::error::Error;
use crate::transport::Transport;

/// The Octen API client. Cloning is cheap and clones share one connection pool.
///
/// ```no_run
/// # async fn run() -> Result<(), octen::Error> {
/// let client = octen::Client::from_env()?; // reads OCTEN_API_KEY
/// # Ok(()) }
/// ```
#[derive(Debug, Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    transport: Transport,
    timeout: Option<Duration>,
}

/// A request body the client can send: it checks its own required inputs before anything is sent.
pub(crate) trait Params: Serialize {
    fn validate(&self) -> Result<(), Error>;
}

impl Client {
    /// A client with this API key and the default settings.
    pub fn new(api_key: impl Into<String>) -> Result<Self, Error> {
        Self::builder().api_key(api_key).build()
    }

    /// A client whose API key comes from the `OCTEN_API_KEY` environment variable.
    pub fn from_env() -> Result<Self, Error> {
        Self::builder().build()
    }

    /// A builder for the base URL, timeout, retries, headers or HTTP client.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    pub(crate) fn from_parts(transport: Transport, timeout: Option<Duration>) -> Self {
        Self {
            inner: Arc::new(Inner { transport, timeout }),
        }
    }

    /// Validates and posts `params`, then decodes the successful envelope into `R`, returning the
    /// raw body too. The attempt timeout is the call's, else the client's, else `default_timeout`.
    pub(crate) async fn post<P: Params, R: DeserializeOwned>(
        &self,
        path: &str,
        params: &P,
        opts: Option<&RequestOptions>,
        default_timeout: Duration,
    ) -> Result<(R, bytes::Bytes), Error> {
        params.validate()?;
        if let Some(o) = opts {
            o.check()?;
        }
        let body = serde_json::to_vec(params).map_err(|e| {
            Error::validation(
                "params",
                format!("The params cannot be encoded as JSON: {e}"),
            )
        })?;
        let timeout = opts
            .and_then(|o| o.timeout)
            .or(self.inner.timeout)
            .unwrap_or(default_timeout);
        let extra = opts.map(|o| o.headers.as_slice()).unwrap_or_default();
        let ok = self
            .inner
            .transport
            .post(path, body.into(), timeout, extra)
            .await?;
        match serde_json::from_slice::<R>(strip_bom(&ok.body)) {
            Ok(r) => Ok((r, ok.body)),
            Err(source) => Err(Error::Decode {
                source,
                request_id: request_id_of_body(&ok.headers, &ok.body),
                body: ok.body,
            }),
        }
    }
}

#[cfg(test)]
mod tests;
