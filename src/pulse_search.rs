//! Pulse Search.

use serde::{Deserialize, Serialize};

use crate::client::Params;
use crate::error::Error;
use crate::types::{
    require_query, string_enum, FullContentOptions, HighlightOptions, Response, Safesearch,
    WebResult,
};

string_enum! {
    /// The Pulse Search mode, which is also what the call is billed at.
    PulseMode { Lite = "lite", Standard = "standard" }
}

/// The request of Pulse Search.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct PulseSearchParams {
    /// The search query; required.
    pub query: String,
    /// The mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<PulseMode>,
    /// The number of results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// Highlight options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<HighlightOptions>,
    /// Full-content options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_content: Option<FullContentOptions>,
    /// Safe search filtering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safesearch: Option<Safesearch>,
    /// Whether to return images on each result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_images: Option<bool>,
}

impl PulseSearchParams {
    /// A request for `query` with every other field unset.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            ..Self::default()
        }
    }
}

impl Params for PulseSearchParams {
    fn validate(&self) -> Result<(), Error> {
        require_query(&self.query)
    }
}

/// The data of a Pulse Search response.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct PulseSearchData {
    /// The query that was run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// The results; a result's title is null for some pages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<WebResult>>,
}

/// What a Pulse Search call is billed for.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct PulseSearchUsage {
    /// The number of searches run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_search_queries: Option<i64>,
    /// The results whose full content was extracted beyond the included amount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_content_extra_count: Option<i64>,
    /// The mode the call is billed at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billed_mode: Option<String>,
}

/// The meta of a Pulse Search response.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct PulseSearchMeta {
    /// What the call is billed for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<PulseSearchUsage>,
    /// Server latency in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency: Option<f64>,
    /// A warning about the request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// The response of Pulse Search.
pub type PulseSearchResponse = Response<PulseSearchData, PulseSearchMeta>;
