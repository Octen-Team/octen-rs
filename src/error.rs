use std::time::Duration;

use bytes::Bytes;

/// Errors returned by the client.
///
/// A request that reached the API and failed is an [`Error::Api`]; inspect the [`ApiError`] for the
/// status, code and message. Invalid input is reported as [`Error::Validation`] before anything is sent.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The API answered with a non-2xx status, or with a 2xx body that is not a successful envelope.
    #[error(transparent)]
    Api(#[from] ApiError),

    /// One attempt exceeded its timeout. It is not retried: the server may already be working on it.
    #[error("octen: request timed out after {timeout:?}")]
    Timeout {
        /// The per-attempt timeout that expired.
        timeout: Duration,
    },

    /// A network failure other than a timeout.
    #[error("octen: connection error: {source}")]
    Connection {
        /// The underlying transport error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Invalid input or configuration, rejected before any request is sent.
    #[error("octen: invalid {field}: {message}")]
    Validation {
        /// The offending field, such as `query`, `input[2]` or `api_key`.
        field: String,
        /// What is wrong with it.
        message: String,
    },

    /// A successful response whose body does not fit the typed response. The call was billed:
    /// do not retry it; the data is in `body`.
    #[error("octen: decode response: {source}")]
    Decode {
        /// The JSON error.
        #[source]
        source: serde_json::Error,
        /// The raw response body.
        body: Bytes,
        /// The request ID, from the `x-request-id` header or the body.
        request_id: Option<String>,
    },
}

impl Error {
    pub(crate) fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Error::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    /// The [`ApiError`] when this is an [`Error::Api`].
    pub fn as_api(&self) -> Option<&ApiError> {
        match self {
            Error::Api(e) => Some(e),
            _ => None,
        }
    }
}

/// A failed API response.
#[derive(Debug, Clone, thiserror::Error)]
#[error("octen: {status} {message}")]
pub struct ApiError {
    /// The HTTP status.
    pub status: u16,
    /// The `code` field of the body when it is a finite number.
    pub code: Option<f64>,
    /// The message from the body (`msg`, `message`, `detail` or `error`), else the start of the raw body.
    pub message: String,
    /// The request ID, from the `x-request-id` header or the body.
    pub request_id: Option<String>,
    /// For a 429, the server's `Retry-After`, rounded up to whole seconds.
    pub retry_after: Option<Duration>,
    /// The raw response body.
    pub body: Bytes,
}

impl ApiError {
    /// 401: the API key is missing or invalid.
    pub fn is_authentication(&self) -> bool {
        self.status == 401
    }

    /// 403: the key may not use this endpoint.
    pub fn is_permission_denied(&self) -> bool {
        self.status == 403
    }

    /// 400 or 422: the request was rejected, for example a value out of range.
    pub fn is_bad_request(&self) -> bool {
        self.status == 400 || self.status == 422
    }

    /// 429: rate limited; see [`ApiError::retry_after`].
    pub fn is_rate_limited(&self) -> bool {
        self.status == 429
    }

    /// 500 or above.
    pub fn is_server_error(&self) -> bool {
        self.status >= 500
    }
}
