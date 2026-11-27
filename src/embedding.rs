//! Embedding and VL Embedding.

use serde::Serialize;

use crate::client::Params;
use crate::error::Error;
use crate::types::{require_finite, response_struct, string_enum, Response};

const MAX_INPUTS: usize = 1000;
const MAX_CONTENTS: usize = 20;

string_enum! {
    /// The text embedding model; the server uses `octen-embedding-4b` when unset.
    EmbeddingModel { OctenEmbedding0_6B = "octen-embedding-0.6b", OctenEmbedding4B = "octen-embedding-4b", OctenEmbedding8B = "octen-embedding-8b" }
}

string_enum! {
    /// `query` adds a retrieval prompt (counted in the input tokens); use `document`, or leave it
    /// unset, for the texts you index.
    InputType { Query = "query", Document = "document" }
}

string_enum! {
    /// The VL embedding model.
    VlEmbeddingModel { OctenVlEmbedding = "octen-vl-embedding", OctenVlEmbeddingLarge = "octen-vl-embedding-large" }
}

/// The request of Embedding: 1 to 1000 texts, none blank.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct EmbeddingParams {
    /// The texts to embed. The server cuts a text longer than 10,000 characters and says so in `meta.warning`.
    pub input: Vec<String>,
    /// The model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<EmbeddingModel>,
    /// The vector size; the model's maximum when unset. The server rejects a larger value with a 400.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimension: Option<i64>,
    /// Whether the texts are queries or documents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_type: Option<InputType>,
}

impl EmbeddingParams {
    /// A request for these texts with every other field unset.
    pub fn new<I, S>(input: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            input: input.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }
}

impl Params for EmbeddingParams {
    // A blank text is rejected here because the server accepts and bills it.
    fn validate(&self) -> Result<(), Error> {
        if self.input.is_empty() || self.input.len() > MAX_INPUTS {
            return Err(Error::validation(
                "input",
                format!("input is required and must hold 1 to {MAX_INPUTS} strings."),
            ));
        }
        for (i, text) in self.input.iter().enumerate() {
            if text.trim().is_empty() {
                let field = format!("input[{i}]");
                return Err(Error::validation(
                    &field,
                    format!("{field} must be a non-empty string."),
                ));
            }
        }
        Ok(())
    }
}

/// One item to embed, serialized as `{"text": …}`, `{"image": …}` or `{"video": …}`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum VlEmbeddingContent {
    /// A text.
    Text(String),
    /// An image as a public URL or base64 data.
    Image(String),
    /// A video as a public URL.
    Video(String),
}

/// The items of a VL Embedding request.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct VlEmbeddingInput {
    /// 1 to 20 items.
    pub contents: Vec<VlEmbeddingContent>,
}

/// The request of VL Embedding: text, images and video in one vector space. The server checks the
/// image and video counts, `dimension` and `fps`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VlEmbeddingParams {
    /// The model.
    pub model: VlEmbeddingModel,
    /// The items.
    pub input: VlEmbeddingInput,
    /// Merge all items into one vector instead of one per item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_fusion: Option<bool>,
    /// The vector size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimension: Option<i64>,
    /// Video frames sampled per second (0 to 1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<f64>,
    /// An instruction for the embedding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruct: Option<String>,
}

impl VlEmbeddingParams {
    /// A request for these items with every other field unset.
    pub fn new(model: VlEmbeddingModel, contents: Vec<VlEmbeddingContent>) -> Self {
        Self {
            model,
            input: VlEmbeddingInput { contents },
            enable_fusion: None,
            dimension: None,
            fps: None,
            instruct: None,
        }
    }
}

impl Params for VlEmbeddingParams {
    // Each item holds exactly one value by construction; it must not be blank, as the API reference says.
    fn validate(&self) -> Result<(), Error> {
        let contents = &self.input.contents;
        if contents.is_empty() || contents.len() > MAX_CONTENTS {
            return Err(Error::validation(
                "input.contents",
                format!("input.contents is required and must hold 1 to {MAX_CONTENTS} items."),
            ));
        }
        for (i, item) in contents.iter().enumerate() {
            let (VlEmbeddingContent::Text(v)
            | VlEmbeddingContent::Image(v)
            | VlEmbeddingContent::Video(v)) = item;
            if v.trim().is_empty() {
                let field = format!("input.contents[{i}]");
                return Err(Error::validation(
                    &field,
                    format!("{field} must be a non-empty string."),
                ));
            }
        }
        require_finite("fps", self.fps)
    }
}

response_struct! {
    /// The vector for one input; `index` is its position in `input`.
    EmbeddingResult {
        /// The position in `input`.
        index: i64,
        /// The vector.
        embedding: Vec<f64>,
    }
}

response_struct! {
    /// The data of an Embedding response.
    EmbeddingData {
        /// One vector per input.
        results: Vec<EmbeddingResult>,
        /// The model used.
        model: String,
    }
}

response_struct! {
    /// What an Embedding call is billed for.
    EmbeddingUsage {
        /// The input tokens.
        input_tokens: i64,
    }
}

response_struct! {
    /// The meta of an Embedding response. `warning` says when the server cut an input longer than 10,000 characters.
    EmbeddingMeta {
        /// What the call is billed for.
        usage: EmbeddingUsage,
        /// Server latency in milliseconds.
        latency: f64,
        /// A warning, such as a truncated input.
        warning: String,
    }
}

/// The response of Embedding.
pub type EmbeddingResponse = Response<EmbeddingData, EmbeddingMeta>;

response_struct! {
    /// One VL vector. `result_type` is `vl` for a per-item vector, whose `index` is the item's position,
    /// or `fusion` for the single fused vector; another value is returned as sent.
    VlEmbeddingResult {
        /// The position of the item.
        index: i64,
        /// The vector.
        embedding: Vec<f64>,
        /// `vl` or `fusion`.
        #[serde(rename = "type")]
        result_type: String,
    }
}

response_struct! {
    /// The data of a VL Embedding response.
    VlEmbeddingData {
        /// The vectors.
        results: Vec<VlEmbeddingResult>,
        /// The model used.
        model: String,
    }
}

response_struct! {
    /// What a VL Embedding call is billed for; `duration` is the video length in seconds.
    VlEmbeddingUsage {
        /// All input tokens.
        input_tokens: i64,
        /// Text tokens.
        text_tokens: i64,
        /// Image tokens.
        image_tokens: i64,
        /// The number of images.
        image_count: i64,
        /// The video length in seconds.
        duration: i64,
    }
}

response_struct! {
    /// The meta of a VL Embedding response.
    VlEmbeddingMeta {
        /// What the call is billed for.
        usage: VlEmbeddingUsage,
        /// A warning about the request.
        warning: String,
    }
}

/// The response of VL Embedding.
pub type VlEmbeddingResponse = Response<VlEmbeddingData, VlEmbeddingMeta>;
