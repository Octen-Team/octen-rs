//! Types shared by several endpoints.

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::error::Error;

/// A string enum serialized as the value in the API reference. `ALL` lists every value, in order.
macro_rules! string_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$vmeta:meta])* $variant:ident = $value:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        #[non_exhaustive]
        pub enum $name {
            $($(#[$vmeta])* #[serde(rename = $value)] $variant),+
        }

        impl $name {
            /// Every value, in the order of the API reference.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            /// The value sent on the wire.
            pub fn as_str(self) -> &'static str {
                match self { $($name::$variant => $value),+ }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}
pub(crate) use string_enum;

/// A response struct whose fields are all optional and omitted when unset, so new or missing fields never break decoding.
macro_rules! response_struct {
    ($(#[$meta:meta])* $name:ident { $($(#[$fmeta:meta])* $field:ident: $ty:ty),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
        #[non_exhaustive]
        pub struct $name {
            $(
                $(#[$fmeta])*
                #[serde(default, skip_serializing_if = "Option::is_none")]
                pub $field: Option<$ty>,
            )+
        }
    };
}
pub(crate) use response_struct;

string_enum! {
    /// How results are returned: `standard` as the request fields specify, or `dynamic`, only relevant
    /// results up to `count`, with the highlight length adjusted per result.
    ResultType { Standard = "standard", Dynamic = "dynamic" }
}

string_enum! {
    /// Which date a time filter applies to.
    TimeBasis { Auto = "auto", Published = "published", Crawled = "crawled" }
}

string_enum! {
    /// A recent time window; the one-letter values are short forms of the others.
    TimeRange { Day = "day", Week = "week", Month = "month", Year = "year", D = "d", W = "w", M = "m", Y = "y" }
}

string_enum! {
    /// A result language.
    Language {
        Ar = "ar", De = "de", En = "en", Es = "es", Fr = "fr", Hi = "hi", Id = "id", It = "it", Ja = "ja",
        Ko = "ko", Nl = "nl", Pl = "pl", Pt = "pt", Ru = "ru", Th = "th", Tr = "tr", Vi = "vi", Zh = "zh",
    }
}

string_enum! {
    /// The format of `full_content`.
    Format { Markdown = "markdown", Text = "text" }
}

string_enum! {
    /// Safe search filtering.
    Safesearch { Off = "off", Strict = "strict" }
}

/// Highlight extraction options.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct HighlightOptions {
    /// Whether to return highlights.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    /// The highlight length cap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,
}

/// Full-content extraction options.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct FullContentOptions {
    /// Whether to return the full content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    /// The full-content length cap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,
}

/// The response envelope shared by every endpoint; each endpoint names it with its own data and meta.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Response<D, M> {
    /// The business code; 0 on success.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<serde_json::Value>,
    /// The status message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
    /// The request ID; include it when contacting support.
    #[serde(default)]
    pub request_id: String,
    /// The endpoint's data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<D>,
    /// Usage, latency and warnings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<M>,
    #[serde(skip)]
    raw: Bytes,
}

impl<D, M> Response<D, M> {
    /// The response body exactly as received, including fields this SDK does not model.
    pub fn raw_json(&self) -> &[u8] {
        &self.raw
    }

    pub(crate) fn with_raw(mut self, raw: Bytes) -> Self {
        self.raw = raw;
        self
    }
}

/// An image attached to a web result.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct ResultImage {
    /// The image URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// A description of the image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// One web result. Search, Web Search Premier, Broad, News, Business, Pulse, Place and Design Search
/// all return results of this shape.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct WebResult {
    /// The page title; Pulse Search sends null for some pages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The page URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The highlight, when highlights are on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight: Option<String>,
    /// The full content, when it was requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_content: Option<String>,
    /// The authors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authors: Option<String>,
    /// When the page was published.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_published: Option<String>,
    /// When the page was last crawled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_last_crawled: Option<String>,
    /// The site icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    /// The cover image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_image: Option<ResultImage>,
    /// Images on the page, when requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ResultImage>>,
}

pub(crate) fn require_query(query: &str) -> Result<(), Error> {
    if query.trim().is_empty() {
        return Err(Error::validation(
            "query",
            "query is required and must be a non-empty string.",
        ));
    }
    Ok(())
}

pub(crate) fn is_blank(s: Option<&str>) -> bool {
    s.is_none_or(|s| s.trim().is_empty())
}

pub(crate) fn require_finite(field: &str, value: Option<f64>) -> Result<(), Error> {
    match value {
        Some(v) if !v.is_finite() => Err(Error::validation(
            field,
            format!("{field} must be a finite number, got {v}."),
        )),
        _ => Ok(()),
    }
}
