//! The resource handles returned by the client's accessors.

use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::business_search::{BusinessSearchParams, BusinessSearchResponse};
use crate::client::{Client, Params};
use crate::config::{RequestOptions, DEFAULT_TIMEOUT};
use crate::design_search::{DesignSearchParams, DesignSearchResponse};
use crate::embedding::{
    EmbeddingParams, EmbeddingResponse, VlEmbeddingParams, VlEmbeddingResponse,
};
use crate::error::Error;
use crate::extract::{ExtractParams, ExtractResponse};
use crate::image_search::{ImageSearchParams, ImageSearchResponse};
use crate::news_search::{NewsSearchParams, NewsSearchResponse};
use crate::place_search::{PlaceSearchParams, PlaceSearchResponse};
use crate::pulse_search::{PulseSearchParams, PulseSearchResponse};
use crate::search::{BroadSearchParams, BroadSearchResponse, SearchParams, SearchResponse};
use crate::types::Response;
use crate::video_search::{VideoSearchParams, VideoSearchResponse};

/// Broad Search fans out into several searches, so it waits longer unless the client or the call sets a timeout.
const BROAD_SEARCH_TIMEOUT: Duration = Duration::from_secs(120);

macro_rules! resource {
    ($(#[$meta:meta])* $name:ident, $accessor:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone)]
        pub struct $name<'a> {
            client: &'a Client,
            options: Option<RequestOptions>,
        }

        impl $name<'_> {
            /// Uses these options, such as a timeout or extra headers, for the calls made through this handle.
            pub fn with_options(mut self, options: RequestOptions) -> Self {
                self.options = Some(options);
                self
            }

            async fn call<P, D, M>(&self, path: &str, params: &P, default_timeout: Duration) -> Result<Response<D, M>, Error>
            where
                P: Params,
                Response<D, M>: DeserializeOwned,
            {
                let (mut resp, raw, header_id) = self
                    .client
                    .post::<P, Response<D, M>>(path, params, self.options.as_ref(), default_timeout)
                    .await?;
                // The body carries the request ID as a rule; the header is the fallback
                if resp.request_id.is_empty() {
                    resp.request_id = header_id.unwrap_or_default();
                }
                Ok(resp.with_raw(raw))
            }
        }

        impl Client {
            #[doc = concat!("The [`", stringify!($name), "`] handle.")]
            pub fn $accessor(&self) -> $name<'_> {
                $name { client: self, options: None }
            }
        }
    };
}

resource! {
    /// Web Search, Web Search Premier, Broad, News, Business and Pulse Search: `client.search()`.
    SearchResource, search
}

resource! {
    /// Place Search: `client.place_search()`.
    PlaceSearchResource, place_search
}

resource! {
    /// Image Search: `client.image_search()`.
    ImageSearchResource, image_search
}

resource! {
    /// Video Search, an invite-only beta: `client.video_search()`.
    VideoSearchResource, video_search
}

resource! {
    /// Design Search: `client.design_search()`.
    DesignSearchResource, design_search
}

resource! {
    /// Extract: `client.extract()`.
    ExtractResource, extract
}

resource! {
    /// Embedding: `client.embedding()`.
    EmbeddingResource, embedding
}

resource! {
    /// VL Embedding: `client.vl_embedding()`.
    VlEmbeddingResource, vl_embedding
}

impl ExtractResource<'_> {
    /// Extract (`POST /extract`): the content of 1 to 20 URLs, one result per URL; match results to
    /// your input by URL. The default timeout is the per-URL `timeout`, clamped to 1 to 60 s (30 s
    /// when unset), plus 90 s, unless the client or the call sets one.
    pub async fn extract(&self, params: &ExtractParams) -> Result<ExtractResponse, Error> {
        self.call("/extract", params, params.default_timeout())
            .await
    }
}

impl EmbeddingResource<'_> {
    /// Embedding (`POST /embedding`): one vector per text; match results to your input by `index`.
    pub async fn create(&self, params: &EmbeddingParams) -> Result<EmbeddingResponse, Error> {
        self.call("/embedding", params, DEFAULT_TIMEOUT).await
    }
}

impl VlEmbeddingResource<'_> {
    /// VL Embedding (`POST /vl-embedding`): one vector per item, or one fused vector with
    /// `enable_fusion`. A long video can take longer than the default 30 s: set a timeout for the call.
    pub async fn create(&self, params: &VlEmbeddingParams) -> Result<VlEmbeddingResponse, Error> {
        self.call("/vl-embedding", params, DEFAULT_TIMEOUT).await
    }
}

impl SearchResource<'_> {
    /// Web Search (`POST /search`).
    pub async fn search(&self, params: &SearchParams) -> Result<SearchResponse, Error> {
        self.call("/search", params, DEFAULT_TIMEOUT).await
    }

    /// Web Search Premier (`POST /search-premier`): deeper retrieval and reranking, with the same
    /// request and response as Web Search.
    pub async fn search_premier(&self, params: &SearchParams) -> Result<SearchResponse, Error> {
        self.call("/search-premier", params, DEFAULT_TIMEOUT).await
    }

    /// Broad Search (`POST /broad-search`): expands the query into several searches. Its default
    /// timeout is 120 s unless the client or the call sets one.
    pub async fn broad_search(
        &self,
        params: &BroadSearchParams,
    ) -> Result<BroadSearchResponse, Error> {
        self.call("/broad-search", params, BROAD_SEARCH_TIMEOUT)
            .await
    }

    /// News Search (`POST /news-search`).
    pub async fn news_search(
        &self,
        params: &NewsSearchParams,
    ) -> Result<NewsSearchResponse, Error> {
        self.call("/news-search", params, DEFAULT_TIMEOUT).await
    }

    /// Business Search (`POST /business-search`).
    pub async fn business_search(
        &self,
        params: &BusinessSearchParams,
    ) -> Result<BusinessSearchResponse, Error> {
        self.call("/business-search", params, DEFAULT_TIMEOUT).await
    }

    /// Pulse Search (`POST /pulse-search`).
    pub async fn pulse_search(
        &self,
        params: &PulseSearchParams,
    ) -> Result<PulseSearchResponse, Error> {
        self.call("/pulse-search", params, DEFAULT_TIMEOUT).await
    }
}

impl PlaceSearchResource<'_> {
    /// Place Search (`POST /place-search`).
    pub async fn search(&self, params: &PlaceSearchParams) -> Result<PlaceSearchResponse, Error> {
        self.call("/place-search", params, DEFAULT_TIMEOUT).await
    }
}

impl ImageSearchResource<'_> {
    /// Image Search (`POST /image-search`).
    pub async fn search(&self, params: &ImageSearchParams) -> Result<ImageSearchResponse, Error> {
        self.call("/image-search", params, DEFAULT_TIMEOUT).await
    }
}

impl VideoSearchResource<'_> {
    /// Video Search (`POST /video-search`), an invite-only beta: a key without access gets a 403.
    pub async fn search(&self, params: &VideoSearchParams) -> Result<VideoSearchResponse, Error> {
        self.call("/video-search", params, DEFAULT_TIMEOUT).await
    }
}

impl DesignSearchResource<'_> {
    /// Design Search (`POST /design-search`).
    pub async fn search(&self, params: &DesignSearchParams) -> Result<DesignSearchResponse, Error> {
        self.call("/design-search", params, DEFAULT_TIMEOUT).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::config::ClientBuilder;
    use crate::embedding::{VlEmbeddingContent, VlEmbeddingModel};
    use crate::transport::Hooks;

    const OK: &str = r#"{"code":0,"request_id":"r","data":{}}"#;

    async fn calls(client: &Client, opts: Option<RequestOptions>) {
        let s = match opts {
            Some(o) => client.search().with_options(o),
            None => client.search(),
        };
        s.search(&SearchParams::new("q")).await.unwrap();
        s.search_premier(&SearchParams::new("q")).await.unwrap();
        s.broad_search(&BroadSearchParams::new("q")).await.unwrap();
        s.news_search(&NewsSearchParams::new("q")).await.unwrap();
        s.business_search(&BusinessSearchParams::new("q"))
            .await
            .unwrap();
        s.pulse_search(&PulseSearchParams::new("q")).await.unwrap();
        client
            .place_search()
            .search(&PlaceSearchParams::new("q"))
            .await
            .unwrap();
        client
            .image_search()
            .search(&ImageSearchParams::text("q"))
            .await
            .unwrap();
        client
            .video_search()
            .search(&VideoSearchParams::text("q"))
            .await
            .unwrap();
        client
            .design_search()
            .search(&DesignSearchParams::new("q"))
            .await
            .unwrap();
    }

    fn client(
        uri: &str,
        configure: impl FnOnce(ClientBuilder) -> ClientBuilder,
    ) -> (Client, Arc<Mutex<Vec<Duration>>>) {
        let timeouts = Arc::new(Mutex::new(Vec::new()));
        let mut b = configure(Client::builder().api_key("k").base_url(uri));
        b.hooks = Hooks {
            timeouts: Some(timeouts.clone()),
            ..Hooks::default()
        };
        (b.build().unwrap(), timeouts)
    }

    #[tokio::test]
    async fn default_timeouts_per_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string(OK))
            .mount(&server)
            .await;
        let s30 = Duration::from_secs(30);

        // Broad Search waits 120 s, every other endpoint 30 s
        let (c, t) = client(&server.uri(), |b| b);
        calls(&c, None).await;
        let want = vec![
            s30,
            s30,
            Duration::from_secs(120),
            s30,
            s30,
            s30,
            s30,
            s30,
            s30,
            s30,
        ];
        assert_eq!(*t.lock().unwrap(), want);

        // A client timeout applies to every endpoint, Broad Search included
        let (c, t) = client(&server.uri(), |b| b.timeout(Duration::from_secs(7)));
        calls(&c, None).await;
        assert_eq!(*t.lock().unwrap(), vec![Duration::from_secs(7); 10]);

        // Extract derives its default from the per-URL timeout, clamped to 1-60 s, plus 90 s
        let (c, t) = client(&server.uri(), |b| b);
        for per_url in [None, Some(60), Some(0), Some(-5), Some(100), Some(10)] {
            let p = ExtractParams {
                timeout: per_url,
                ..ExtractParams::new(["https://a.com"])
            };
            c.extract().extract(&p).await.unwrap();
        }
        c.embedding()
            .create(&EmbeddingParams::new(["a"]))
            .await
            .unwrap();
        let vl = VlEmbeddingParams::new(
            VlEmbeddingModel::OctenVlEmbedding,
            vec![VlEmbeddingContent::Text("a".into())],
        );
        c.vl_embedding().create(&vl).await.unwrap();
        let secs: Vec<u64> = t.lock().unwrap().iter().map(Duration::as_secs).collect();
        assert_eq!(secs, vec![120, 150, 91, 91, 150, 100, 30, 30]);

        // A per-call timeout overrides both, on the handle it is set on
        let opts = RequestOptions::new().timeout(Duration::from_secs(3));
        let (c, t) = client(&server.uri(), |b| b.timeout(Duration::from_secs(7)));
        calls(&c, Some(opts)).await;
        let got = t.lock().unwrap().clone();
        assert_eq!(got[..6], [Duration::from_secs(3); 6]);
        assert_eq!(got[6..], [Duration::from_secs(7); 4]);
    }
}
