//! Video Search, an invite-only beta.

use serde::Serialize;

use crate::client::Params;
use crate::error::Error;
use crate::types::{response_struct, Response, Safesearch, TimeRange};

/// The input of a Video Search; only text is supported.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum VideoSearchInput {
    /// Search by text: `{"type": "text", "data": …}`.
    #[serde(rename = "text")]
    Text {
        /// The query.
        data: String,
    },
}

/// The request of Video Search. `inputs` must hold exactly one text input; [`VideoSearchParams::text`] sets it.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct VideoSearchParams {
    /// Exactly one text input.
    pub inputs: Vec<VideoSearchInput>,
    /// The number of results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// A recent time window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<TimeRange>,
    /// The start of a time window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    /// The end of a time window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    /// Safe search filtering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safesearch: Option<Safesearch>,
}

impl VideoSearchParams {
    /// A search by text.
    pub fn text(query: impl Into<String>) -> Self {
        Self {
            inputs: vec![VideoSearchInput::Text { data: query.into() }],
            ..Self::default()
        }
    }
}

impl Params for VideoSearchParams {
    fn validate(&self) -> Result<(), Error> {
        let [VideoSearchInput::Text { data }] = self.inputs.as_slice() else {
            return Err(Error::validation(
                "inputs",
                "inputs must hold exactly one text input.",
            ));
        };
        if data.trim().is_empty() {
            return Err(Error::validation(
                "inputs[0].data",
                "A text input needs a non-empty data string.",
            ));
        }
        Ok(())
    }
}

response_struct! {
    /// One video result.
    VideoSearchResult {
        /// The title.
        title: String,
        /// The video URL.
        url: String,
        /// The page the video is on.
        source_page: String,
        /// A description.
        description: String,
        /// The cover image URL.
        cover_url: String,
        /// The duration in seconds.
        duration_seconds: i64,
        /// The authors.
        authors: String,
        /// When it was published.
        time_published: String,
        /// When it was last crawled.
        time_last_crawled: String,
    }
}

response_struct! {
    /// The data of a Video Search response.
    VideoSearchData {
        /// The results.
        results: Vec<VideoSearchResult>,
    }
}

response_struct! {
    /// What a Video Search call is billed for.
    VideoSearchUsage {
        /// The number of searches run.
        num_search_queries: i64,
    }
}

response_struct! {
    /// The meta of a Video Search response.
    VideoSearchMeta {
        /// What the call is billed for.
        usage: VideoSearchUsage,
        /// Server latency in milliseconds.
        latency: f64,
        /// A warning about the request.
        warning: String,
    }
}

/// The response of Video Search.
pub type VideoSearchResponse = Response<VideoSearchData, VideoSearchMeta>;
