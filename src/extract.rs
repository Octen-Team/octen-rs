//! Extract.

use std::time::Duration;

use serde::Serialize;

use crate::client::Params;
use crate::error::Error;
use crate::types::{response_struct, string_enum, Format, Response};

const MAX_URLS: usize = 20;
const MAX_URL_CHARS: usize = 2048;
const PER_URL_DEFAULT_SECS: i64 = 30;
const PER_URL_MIN_SECS: i64 = 1;
const PER_URL_MAX_SECS: i64 = 60;
const HEADROOM: Duration = Duration::from_secs(90);

string_enum! {
    /// How pages are fetched: `standard` (the default), `advanced` (slower, more expensive, for pages
    /// that are hard to reach) or `auto` (picks one per URL).
    ExtractMode { Standard = "standard", Advanced = "advanced", Auto = "auto" }
}

string_enum! {
    /// Which links to prefer.
    ExtractLinkScope { PreferInternal = "prefer_internal", PreferExternal = "prefer_external" }
}

/// Asks for the links found on each page. `ExtractLinksOptions::default()` is sent as `{}` and uses
/// the server defaults.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ExtractLinksOptions {
    /// Which links to prefer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<ExtractLinkScope>,
    /// The most links per page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_links: Option<i64>,
}

/// The request of Extract. `urls` must hold 1 to 20 URLs, each non-blank and at most 2048 characters.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ExtractParams {
    /// The URLs to extract.
    pub urls: Vec<String>,
    /// How pages are fetched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ExtractMode>,
    /// Returns highlights relevant to this query instead of the full content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// The oldest cached copy to accept, in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age_seconds: Option<i64>,
    /// The content format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<Format>,
    /// The per-URL time limit on the server in seconds (1 to 60, 30 by default). It is not the HTTP
    /// timeout of the call, which the SDK derives from it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i64>,
    /// Whether to return the images on each page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_images: Option<bool>,
    /// Whether to return the videos on each page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_videos: Option<bool>,
    /// Whether to return the audio on each page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_audio: Option<bool>,
    /// Whether and how to return the links on each page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_links: Option<ExtractLinksOptions>,
}

impl ExtractParams {
    /// A request for these URLs with every other field unset.
    pub fn new<I, S>(urls: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            urls: urls.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    /// The SDK's default timeout for this call: the per-URL timeout, clamped to 1 to 60 s like the
    /// server does (30 s when unset), plus 90 s of headroom for a whole batch.
    pub(crate) fn default_timeout(&self) -> Duration {
        let secs = self.timeout.map_or(PER_URL_DEFAULT_SECS, |t| {
            t.clamp(PER_URL_MIN_SECS, PER_URL_MAX_SECS)
        });
        Duration::from_secs(secs.unsigned_abs()) + HEADROOM
    }
}

impl Params for ExtractParams {
    // URL length is counted in Unicode code points, like the TypeScript and Go SDKs.
    fn validate(&self) -> Result<(), Error> {
        if self.urls.is_empty() || self.urls.len() > MAX_URLS {
            return Err(Error::validation(
                "urls",
                format!("urls is required and must hold 1 to {MAX_URLS} URLs."),
            ));
        }
        for (i, url) in self.urls.iter().enumerate() {
            let field = format!("urls[{i}]");
            if url.trim().is_empty() {
                return Err(Error::validation(
                    &field,
                    format!("{field} must be a non-empty string."),
                ));
            }
            if url.chars().count() > MAX_URL_CHARS {
                return Err(Error::validation(
                    &field,
                    format!("{field} is longer than {MAX_URL_CHARS} characters."),
                ));
            }
        }
        Ok(())
    }
}

response_struct! {
    /// An image, video or audio resource found on a page.
    ExtractMediaResource {
        /// The URL.
        url: String,
    }
}

response_struct! {
    /// A link found on a page.
    ExtractLink {
        /// The URL.
        url: String,
        /// The anchor text.
        anchor_text: String,
        /// Whether it points to another site.
        is_external: bool,
    }
}

response_struct! {
    /// A detected classification: the page structure or the content category. `secondary` is unset
    /// when no sub-type applies.
    ExtractClassification {
        /// The main type.
        primary: String,
        /// The sub-type.
        secondary: String,
    }
}

response_struct! {
    /// The result for one URL. `status` is `success` or `failed`; a failed URL carries `error_message`
    /// and is not billed. `resolved_mode` is the mode a successful URL is billed at. Match results to
    /// your input by `url`.
    ExtractResult {
        /// The URL.
        url: String,
        /// `success` or `failed`.
        status: String,
        /// The mode the URL is billed at.
        resolved_mode: String,
        /// The page title.
        title: String,
        /// The content; with a `query`, `highlights` are returned instead.
        full_content: String,
        /// Highlights relevant to the query.
        highlights: Vec<String>,
        /// When the page was published.
        time_published: String,
        /// When the page was last crawled.
        time_last_crawled: String,
        /// The page structure.
        page_structure: ExtractClassification,
        /// The content category.
        category: ExtractClassification,
        /// The site icon.
        favicon: String,
        /// The cover image.
        cover_image: ExtractMediaResource,
        /// Images on the page.
        images: Vec<ExtractMediaResource>,
        /// Videos on the page.
        videos: Vec<ExtractMediaResource>,
        /// Audio on the page.
        audio: Vec<ExtractMediaResource>,
        /// Links on the page.
        links: Vec<ExtractLink>,
        /// Why a failed URL failed.
        error_message: String,
    }
}

response_struct! {
    /// The data of an Extract response.
    ExtractData {
        /// One result per URL.
        results: Vec<ExtractResult>,
    }
}

response_struct! {
    /// The successful URLs by the mode they are billed at.
    ExtractSuccessfulByMode {
        /// URLs billed at standard.
        standard_urls: i64,
        /// URLs billed at advanced.
        advanced_urls: i64,
    }
}

response_struct! {
    /// What an Extract call is billed for; failed URLs are not counted as successful.
    ExtractUsage {
        /// The URLs requested.
        total_urls: i64,
        /// The URLs extracted.
        successful_urls: i64,
        /// The successful URLs by billed mode.
        successful_by_mode: ExtractSuccessfulByMode,
    }
}

response_struct! {
    /// The meta of an Extract response.
    ExtractMeta {
        /// What the call is billed for.
        usage: ExtractUsage,
        /// Server latency in milliseconds.
        latency: f64,
        /// A warning about the request, such as an adjusted value.
        warning: String,
    }
}

/// The response of Extract.
pub type ExtractResponse = Response<ExtractData, ExtractMeta>;
