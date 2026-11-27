//! Design Search.

use serde::Serialize;

use crate::client::Params;
use crate::error::Error;
use crate::types::{
    is_blank, response_struct, FullContentOptions, HighlightOptions, Language, Response, TimeBasis,
    TimeRange, WebResult,
};

/// Options for the reusable code returned with image results that have it.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct DesignSnippetOptions {
    /// Whether to return snippets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    /// The snippet length cap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,
}

/// Options for the image results of a Design Search. `group_id` returns the images of that group in
/// order instead of searching by query; it takes the `group.group_id` of an image result.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct DesignImageOptions {
    /// Whether to return image results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    /// The number of image results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// Whether to return design facts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub design_facts: Option<bool>,
    /// Snippet options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<DesignSnippetOptions>,
    /// A whole page or deck, in order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
}

/// Options for the web results of a Design Search; they do not affect the image results.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct DesignWebSearchOptions {
    /// Whether to return web results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    /// The number of web results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// Highlight options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<HighlightOptions>,
    /// Full-content options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_content: Option<FullContentOptions>,
    /// Only results from these domains.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub include_domains: Vec<String>,
    /// No results from these domains.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclude_domains: Vec<String>,
    /// Result languages.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub language: Vec<Language>,
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
}

/// The request of Design Search. `query` is required unless `images.group_id` is set.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct DesignSearchParams {
    /// The search query.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// Image result options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<DesignImageOptions>,
    /// Web result options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search: Option<DesignWebSearchOptions>,
}

impl DesignSearchParams {
    /// A search for `query`.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: Some(query.into()),
            ..Self::default()
        }
    }

    /// The images of one group (a whole page or deck), in order, without a query.
    pub fn group(group_id: impl Into<String>) -> Self {
        Self {
            images: Some(DesignImageOptions {
                group_id: Some(group_id.into()),
                ..DesignImageOptions::default()
            }),
            ..Self::default()
        }
    }
}

impl Params for DesignSearchParams {
    fn validate(&self) -> Result<(), Error> {
        let group = self.images.as_ref().and_then(|i| i.group_id.as_deref());
        if is_blank(self.query.as_deref()) && is_blank(group) {
            return Err(Error::validation(
                "query",
                "query is required unless images.group_id is set, and must be a non-empty string.",
            ));
        }
        Ok(())
    }
}

response_struct! {
    /// Where an image result sits in its page or deck: `position` counts from 1 to `total`.
    DesignImageGroup {
        /// The group ID, for `images.group_id`.
        group_id: String,
        /// The kind of group.
        #[serde(rename = "type")]
        group_type: String,
        /// The group name.
        name: String,
        /// The position, from 1.
        position: i64,
        /// The number of images in the group.
        total: i64,
        /// Group-level design values decided by the server.
        master: serde_json::Map<String, serde_json::Value>,
    }
}

response_struct! {
    /// Reusable code for an image result.
    DesignSnippet {
        /// The code format.
        format: String,
        /// The code.
        code: String,
    }
}

response_struct! {
    /// One design reference: a web page, page section, app screen or slide. `design_facts` holds the
    /// design values decided by the server.
    DesignImageResult {
        /// The kind of reference.
        #[serde(rename = "type")]
        image_type: String,
        /// The title.
        title: String,
        /// A description.
        description: String,
        /// The image URL.
        url: String,
        /// A thumbnail URL.
        thumbnail: String,
        /// The source page.
        source_page: String,
        /// The width in pixels.
        width: i64,
        /// The height in pixels.
        height: i64,
        /// Its place in a page or deck.
        group: DesignImageGroup,
        /// Design values.
        design_facts: serde_json::Map<String, serde_json::Value>,
        /// Reusable code.
        snippet: DesignSnippet,
    }
}

response_struct! {
    /// The data of a Design Search response.
    DesignSearchData {
        /// The query that was run.
        query: String,
        /// The image results.
        images: Vec<DesignImageResult>,
        /// The web results.
        results: Vec<WebResult>,
    }
}

response_struct! {
    /// What a Design Search call is billed for.
    DesignSearchUsage {
        /// The number of searches run.
        num_search_queries: i64,
    }
}

response_struct! {
    /// The meta of a Design Search response.
    DesignSearchMeta {
        /// What the call is billed for.
        usage: DesignSearchUsage,
        /// Server latency in milliseconds.
        latency: f64,
        /// A warning about the request.
        warning: String,
    }
}

/// The response of Design Search.
pub type DesignSearchResponse = Response<DesignSearchData, DesignSearchMeta>;
