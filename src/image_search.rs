//! Image Search.

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

use crate::client::Params;
use crate::error::Error;
use crate::types::{response_struct, Response, Safesearch};

/// The input of an Image Search: a text query, or an image given as a public URL or base64 data.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ImageSearchInput {
    /// Search by text: `{"type": "text", "data": …}`.
    Text(String),
    /// Search by an image at a public URL: `{"type": "image", "url": …}`.
    ImageUrl(String),
    /// Search by base64-encoded image data: `{"type": "image", "data": …}`.
    ImageData(String),
}

impl Serialize for ImageSearchInput {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (kind, key, value) = match self {
            Self::Text(v) => ("text", "data", v),
            Self::ImageUrl(v) => ("image", "url", v),
            Self::ImageData(v) => ("image", "data", v),
        };
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("type", kind)?;
        map.serialize_entry(key, value)?;
        map.end()
    }
}

/// The request of Image Search. `inputs` must hold exactly one input; the constructors set it.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ImageSearchParams {
    /// Exactly one input.
    pub inputs: Vec<ImageSearchInput>,
    /// The number of results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// Only images from these domains.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub include_domains: Vec<String>,
    /// No images from these domains.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclude_domains: Vec<String>,
    /// Safe search filtering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safesearch: Option<Safesearch>,
}

impl ImageSearchParams {
    /// A search by text.
    pub fn text(query: impl Into<String>) -> Self {
        Self::with_input(ImageSearchInput::Text(query.into()))
    }

    /// A search by an image at a public URL.
    pub fn image_url(url: impl Into<String>) -> Self {
        Self::with_input(ImageSearchInput::ImageUrl(url.into()))
    }

    /// A search by base64-encoded image data.
    pub fn image_data(base64: impl Into<String>) -> Self {
        Self::with_input(ImageSearchInput::ImageData(base64.into()))
    }

    fn with_input(input: ImageSearchInput) -> Self {
        Self {
            inputs: vec![input],
            ..Self::default()
        }
    }
}

impl Params for ImageSearchParams {
    fn validate(&self) -> Result<(), Error> {
        let [input] = self.inputs.as_slice() else {
            return Err(Error::validation(
                "inputs",
                "inputs must hold exactly one input: one text input or one image input.",
            ));
        };
        match input {
            ImageSearchInput::Text(data) if data.trim().is_empty() => {
                Err(Error::validation("inputs[0].data", "A text input needs a non-empty data string."))
            }
            ImageSearchInput::ImageUrl(v) | ImageSearchInput::ImageData(v) if v.trim().is_empty() => Err(
                Error::validation("inputs[0]", "An image input needs a non-empty url (a public URL) or data (base64 image data)."),
            ),
            _ => Ok(()),
        }
    }
}

response_struct! {
    /// One image result.
    ImageSearchResult {
        /// The title.
        title: String,
        /// The image URL.
        url: String,
        /// The page the image is on.
        source_page: String,
        /// A description.
        description: String,
        /// The width in pixels.
        width: i64,
        /// The height in pixels.
        height: i64,
        /// A thumbnail URL.
        thumbnail: String,
    }
}

response_struct! {
    /// The data of an Image Search response.
    ImageSearchData {
        /// The results.
        results: Vec<ImageSearchResult>,
    }
}

response_struct! {
    /// What an Image Search call is billed for.
    ImageSearchUsage {
        /// The number of searches run.
        num_search_queries: i64,
    }
}

response_struct! {
    /// The meta of an Image Search response.
    ImageSearchMeta {
        /// What the call is billed for.
        usage: ImageSearchUsage,
        /// Server latency in milliseconds.
        latency: f64,
        /// A warning about the request.
        warning: String,
    }
}

/// The response of Image Search.
pub type ImageSearchResponse = Response<ImageSearchData, ImageSearchMeta>;
