//! News Search.

use serde::{Deserialize, Serialize};

use crate::client::Params;
use crate::error::Error;
use crate::types::{
    require_query, FullContentOptions, HighlightOptions, Language, Response, ResultImage,
    TimeBasis, TimeRange, WebResult,
};

/// Subject clustering options of a News Search.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct NewsSubjectOptions {
    /// Whether to group results into subjects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    /// The number of subjects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// The most supporting articles per subject.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_sub_news: Option<i64>,
}

/// The request of News Search.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct NewsSearchParams {
    /// The search query; required.
    pub query: String,
    /// The number of results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// Only results from these domains.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub include_domains: Vec<String>,
    /// No results from these domains.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclude_domains: Vec<String>,
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
    /// Subject clustering options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subjects: Option<NewsSubjectOptions>,
}

impl NewsSearchParams {
    /// A request for `query` with every other field unset.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            ..Self::default()
        }
    }
}

impl Params for NewsSearchParams {
    fn validate(&self) -> Result<(), Error> {
        require_query(&self.query)
    }
}

/// An article in a news subject. The lead article also has a summary.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct NewsArticle {
    /// The title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// A summary; set on the lead article.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// The highlight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight: Option<String>,
    /// The full content, when requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_content: Option<String>,
    /// The authors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authors: Option<String>,
    /// When it was published.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_published: Option<String>,
    /// When it was last crawled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_last_crawled: Option<String>,
    /// The site icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    /// The cover image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_image: Option<ResultImage>,
    /// Images in the article.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ResultImage>>,
}

/// A cluster of articles about one story.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct NewsSubject {
    /// The subject name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// A summary of the story.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// When the story started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_start: Option<String>,
    /// The latest update.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_latest: Option<String>,
    /// The lead article.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_news: Option<NewsArticle>,
    /// Supporting articles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_news: Option<Vec<NewsArticle>>,
}

/// The data of a News Search response.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct NewsSearchData {
    /// The query that was run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// The results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<WebResult>>,
    /// The subjects, when clustering is on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subjects: Option<Vec<NewsSubject>>,
}

/// What a News Search call is billed for.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct NewsSearchUsage {
    /// The number of searches run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_search_queries: Option<i64>,
    /// The number of subject searches run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_subject_search_queries: Option<i64>,
    /// The results whose full content was extracted beyond the included amount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_content_extra_count: Option<i64>,
}

/// The meta of a News Search response.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct NewsSearchMeta {
    /// What the call is billed for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<NewsSearchUsage>,
    /// Server latency in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency: Option<f64>,
    /// A warning about the request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// The response of News Search.
pub type NewsSearchResponse = Response<NewsSearchData, NewsSearchMeta>;
