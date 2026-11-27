//! Web Search, Web Search Premier and Broad Search.

use serde::{Deserialize, Serialize};

use crate::client::Params;
use crate::error::Error;
use crate::types::{
    require_query, Format, FullContentOptions, HighlightOptions, Language, Response, ResultType,
    Safesearch, TimeBasis, TimeRange, WebResult,
};

/// The request of Web Search and Web Search Premier. Build it with [`SearchParams::new`] and set
/// the fields you need: `SearchParams { count: Some(5), ..SearchParams::new("rust async runtimes") }`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SearchParams {
    /// The search query; required.
    pub query: String,
    /// How results are returned; `standard` by default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_type: Option<ResultType>,
    /// The number of results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// Only results from these domains.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub include_domains: Vec<String>,
    /// No results from these domains.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclude_domains: Vec<String>,
    /// Only results containing these texts.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub include_text: Vec<String>,
    /// No results containing these texts.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclude_text: Vec<String>,
    /// Which date the time filters apply to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_basis: Option<TimeBasis>,
    /// A recent time window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<TimeRange>,
    /// The start of a time window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    /// The end of a time window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    /// Result languages.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub language: Vec<Language>,
    /// Highlight options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<HighlightOptions>,
    /// Full-content options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_content: Option<FullContentOptions>,
    /// The format of the full content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<Format>,
    /// Safe search filtering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safesearch: Option<Safesearch>,
    /// Whether to return images on each result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_images: Option<bool>,
}

impl SearchParams {
    /// A request for `query` with every other field unset.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            ..Self::default()
        }
    }
}

impl Params for SearchParams {
    fn validate(&self) -> Result<(), Error> {
        require_query(&self.query)
    }
}

/// The web search options applied to every expanded query of a Broad Search.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct WebSearchOptions {
    /// How results are returned; `standard` by default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_type: Option<ResultType>,
    /// The number of results per query.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// Only results from these domains.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub include_domains: Vec<String>,
    /// No results from these domains.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclude_domains: Vec<String>,
    /// Only results containing these texts.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub include_text: Vec<String>,
    /// No results containing these texts.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclude_text: Vec<String>,
    /// Which date the time filters apply to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_basis: Option<TimeBasis>,
    /// A recent time window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<TimeRange>,
    /// The start of a time window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    /// The end of a time window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    /// Result languages.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub language: Vec<Language>,
    /// Highlight options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<HighlightOptions>,
    /// Full-content options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_content: Option<FullContentOptions>,
    /// The format of the full content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<Format>,
    /// Safe search filtering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safesearch: Option<Safesearch>,
    /// Whether to return images on each result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_images: Option<bool>,
}

/// The request of Broad Search, which expands the query into several web searches.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct BroadSearchParams {
    /// The search query; required.
    pub query: String,
    /// The most expanded queries to run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_queries: Option<i64>,
    /// Options for each expanded query.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_options: Option<WebSearchOptions>,
}

impl BroadSearchParams {
    /// A request for `query` with every other field unset.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            ..Self::default()
        }
    }
}

impl Params for BroadSearchParams {
    fn validate(&self) -> Result<(), Error> {
        require_query(&self.query)
    }
}

/// The data of a Web Search or Web Search Premier response.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct SearchData {
    /// The query that was run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// The results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<WebResult>>,
}

/// What a search call is billed for.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct SearchUsage {
    /// The number of searches run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_search_queries: Option<i64>,
    /// The results whose full content was extracted beyond the included amount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_content_extra_count: Option<i64>,
}

/// The meta of a Web Search, Web Search Premier or Broad Search response.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct SearchMeta {
    /// What the call is billed for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<SearchUsage>,
    /// The result type used; Web Search sends it, Broad Search does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_type: Option<String>,
    /// Server latency in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency: Option<f64>,
    /// A warning about the request, such as an adjusted value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// The response of Web Search and Web Search Premier.
pub type SearchResponse = Response<SearchData, SearchMeta>;

/// The results of one expanded query of a Broad Search.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct SearchResultGroup {
    /// The expanded query.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// Its results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<WebResult>>,
    /// Its latency in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency: Option<f64>,
}

/// The data of a Broad Search response.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct BroadSearchData {
    /// The original query.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// The expanded queries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queries: Option<Vec<String>>,
    /// The results of each expanded query.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_results: Option<Vec<SearchResultGroup>>,
}

/// The response of Broad Search.
pub type BroadSearchResponse = Response<BroadSearchData, SearchMeta>;
