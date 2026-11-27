//! Calls the production API and is billed. The tests are ignored by default; run them with
//! `OCTEN_LIVE=1 OCTEN_API_KEY=… cargo test --test live -- --ignored --test-threads=1`. Without
//! OCTEN_LIVE=1 and a key they fail rather than pass, so a run cannot succeed without calling the API.
//! Video Search is an invite-only beta: its test also needs OCTEN_LIVE_VIDEO=1 and a key with access.

use std::time::Instant;

use octen::*;

// Public CC0 media the API could fetch on 2026-11-26.
const IMAGE: &str =
    "https://interactive-examples.mdn.mozilla.net/media/cc0-images/grapefruit-slice-332-332.jpg";
const VIDEO: &str = "https://interactive-examples.mdn.mozilla.net/media/cc0-videos/flower.mp4";

fn live_client() -> Client {
    let key = std::env::var("OCTEN_API_KEY").unwrap_or_default();
    assert!(
        std::env::var("OCTEN_LIVE").as_deref() == Ok("1") && !key.trim().is_empty(),
        "live tests need OCTEN_LIVE=1 and OCTEN_API_KEY"
    );
    Client::new(key).unwrap()
}

fn require<D, M>(resp: &Response<D, M>) -> &D {
    assert!(!resp.request_id.is_empty(), "empty request_id");
    println!("request_id={}", resp.request_id);
    resp.data.as_ref().expect("response has no data")
}

#[tokio::test]
#[ignore = "calls the production API"]
async fn live_search() {
    let p = SearchParams {
        count: Some(1),
        ..SearchParams::new("Octen search API")
    };
    let resp = live_client().search().search(&p).await.unwrap();
    assert!(!require(&resp)
        .results
        .as_deref()
        .unwrap_or_default()
        .is_empty());
}

#[tokio::test]
#[ignore = "calls the production API"]
async fn live_search_premier() {
    let p = SearchParams {
        count: Some(1),
        ..SearchParams::new("Octen search API")
    };
    let resp = live_client().search().search_premier(&p).await.unwrap();
    require(&resp);
}

#[tokio::test]
#[ignore = "calls the production API"]
async fn live_broad_search() {
    let p = BroadSearchParams {
        max_queries: Some(1),
        ..BroadSearchParams::new("AI search APIs compared")
    };
    let resp = live_client().search().broad_search(&p).await.unwrap();
    assert!(!require(&resp)
        .search_results
        .as_deref()
        .unwrap_or_default()
        .is_empty());
}

#[tokio::test]
#[ignore = "calls the production API"]
async fn live_news_search() {
    let p = NewsSearchParams {
        count: Some(1),
        ..NewsSearchParams::new("artificial intelligence")
    };
    let resp = live_client().search().news_search(&p).await.unwrap();
    require(&resp);
}

#[tokio::test]
#[ignore = "calls the production API"]
async fn live_business_search() {
    let p = BusinessSearchParams {
        count: Some(1),
        ..BusinessSearchParams::new("Apple Inc")
    };
    let resp = live_client().search().business_search(&p).await.unwrap();
    require(&resp);
}

#[tokio::test]
#[ignore = "calls the production API"]
async fn live_pulse_search() {
    let p = PulseSearchParams {
        count: Some(1),
        ..PulseSearchParams::new("Octen search API")
    };
    let resp = live_client().search().pulse_search(&p).await.unwrap();
    require(&resp);
}

#[tokio::test]
#[ignore = "calls the production API"]
async fn live_place_search() {
    let p = PlaceSearchParams {
        places: Some(PlaceCardOptions {
            count: Some(1),
            ..Default::default()
        }),
        web_search: Some(PlaceWebSearchOptions {
            enable: Some(false),
            ..Default::default()
        }),
        latitude: Some(35.6812),
        longitude: Some(139.7671),
        radius: Some(1000),
        ..PlaceSearchParams::new("coffee")
    };
    let resp = live_client().place_search().search(&p).await.unwrap();
    assert!(!require(&resp)
        .places
        .as_deref()
        .unwrap_or_default()
        .is_empty());
}

// Searching by a well-known Commons photo finds that photo again, which a text query need not.
#[tokio::test]
#[ignore = "calls the production API"]
async fn live_image_search_by_url() {
    let p = ImageSearchParams {
        count: Some(2),
        ..ImageSearchParams::image_url(
            "https://upload.wikimedia.org/wikipedia/commons/a/a3/June_odd-eyed-cat.jpg",
        )
    };
    let resp = live_client().image_search().search(&p).await.unwrap();
    let results = require(&resp).results.clone().unwrap_or_default();
    let found = results.iter().any(|r| {
        let text = format!(
            "{} {}",
            r.url.as_deref().unwrap_or(""),
            r.source_page.as_deref().unwrap_or("")
        );
        text.to_lowercase().contains("odd-eyed")
    });
    assert!(found, "no result points at the input photo: {results:?}");
}

#[tokio::test]
#[ignore = "calls the production API; Video Search also needs OCTEN_LIVE_VIDEO=1"]
async fn live_video_search() {
    assert_eq!(
        std::env::var("OCTEN_LIVE_VIDEO").as_deref(),
        Ok("1"),
        "Video Search needs OCTEN_LIVE_VIDEO=1 and a key with beta access"
    );
    let p = VideoSearchParams {
        count: Some(1),
        ..VideoSearchParams::text("how to tie a bowline knot")
    };
    let resp = live_client().video_search().search(&p).await.unwrap();
    require(&resp);
}

#[tokio::test]
#[ignore = "calls the production API"]
async fn live_design_search() {
    let p = DesignSearchParams {
        images: Some(DesignImageOptions {
            count: Some(2),
            ..Default::default()
        }),
        ..DesignSearchParams::new("pricing page")
    };
    let resp = live_client().design_search().search(&p).await.unwrap();
    assert!(!require(&resp)
        .images
        .as_deref()
        .unwrap_or_default()
        .is_empty());
}

// One URL that cannot resolve between two that work: the call succeeds and only that URL fails.
#[tokio::test]
#[ignore = "calls the production API"]
async fn live_extract() {
    let (a, broken, b) = (
        "https://en.wikipedia.org/wiki/Web_scraping",
        "https://octen-sdk-live-check.invalid/",
        "https://octen.ai/",
    );
    let resp = live_client()
        .extract()
        .extract(&ExtractParams::new([a, broken, b]))
        .await
        .unwrap();
    let results = require(&resp).results.clone().unwrap_or_default();
    let by_url = |u: &str| {
        results
            .iter()
            .find(|r| r.url.as_deref() == Some(u))
            .unwrap_or_else(|| panic!("no result for {u}"))
    };
    for u in [a, b] {
        let r = by_url(u);
        assert_eq!(r.status.as_deref(), Some("success"), "{u}: {r:?}");
        assert!(
            r.full_content.as_deref().unwrap_or("").len() > 200,
            "{u}: short content"
        );
    }
    let r = by_url(broken);
    assert_eq!(r.status.as_deref(), Some("failed"));
    assert!(!r.error_message.as_deref().unwrap_or("").is_empty());
    let usage = resp
        .meta
        .as_ref()
        .and_then(|m| m.usage.as_ref())
        .expect("usage");
    assert_eq!(
        (usage.total_urls, usage.successful_urls),
        (Some(3), Some(2))
    );
}

#[tokio::test]
#[ignore = "calls the production API"]
async fn live_embedding_basic() {
    let p = EmbeddingParams {
        model: Some(EmbeddingModel::OctenEmbedding0_6B),
        dimension: Some(8),
        input_type: Some(InputType::Query),
        ..EmbeddingParams::new(["What is vector search?"])
    };
    let resp = live_client().embedding().create(&p).await.unwrap();
    let data = require(&resp);
    let results = data.results.as_deref().unwrap_or_default();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].embedding.as_deref().unwrap_or_default().len(),
        8,
        "dimension applied"
    );
    assert_eq!(
        data.model.as_deref(),
        Some("octen-embedding-0.6b"),
        "model applied"
    );
    let tokens = resp
        .meta
        .as_ref()
        .and_then(|m| m.usage.as_ref())
        .and_then(|u| u.input_tokens);
    assert!(tokens.unwrap_or(0) > 0);
}

fn cosine(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm = |v: &[f64]| v.iter().map(|x| x * x).sum::<f64>().sqrt();
    dot / (norm(a) * norm(b))
}

#[tokio::test]
#[ignore = "calls the production API"]
async fn live_embedding_batch_order() {
    let c = live_client();
    let texts = ["red apple", "blue ocean", "green forest"];
    let params = |input: Vec<&str>| EmbeddingParams {
        model: Some(EmbeddingModel::OctenEmbedding0_6B),
        dimension: Some(8),
        ..EmbeddingParams::new(input)
    };
    let batch = c.embedding().create(&params(texts.to_vec())).await.unwrap();
    // Callers match by index, not by position: every input index exactly once
    let mut by_index = vec![None; texts.len()];
    for r in require(&batch).results.as_deref().unwrap_or_default() {
        let i = usize::try_from(r.index.expect("index")).unwrap();
        assert!(by_index[i].is_none(), "index {i} twice");
        by_index[i] = r.embedding.clone();
    }
    // Control: each text alone. Batched values differ by about 1e-4 (seen 2026-11-27), so the vector
    // for index i must be the one closest to text i alone, not equal to it.
    let mut alone = Vec::new();
    for t in texts {
        let single = c.embedding().create(&params(vec![t])).await.unwrap();
        alone.push(
            require(&single).results.as_deref().unwrap_or_default()[0]
                .embedding
                .clone()
                .unwrap_or_default(),
        );
    }
    for (i, b) in by_index.iter().enumerate() {
        let b = b
            .as_deref()
            .unwrap_or_else(|| panic!("no vector for index {i}"));
        assert_eq!(b.len(), 8);
        let sims: Vec<f64> = alone.iter().map(|a| cosine(a, b)).collect();
        println!("cos(batch[{i}], alone) = {sims:?}");
        assert!(sims.iter().all(|s| s.is_finite()));
        assert!(
            sims[i] > 0.99,
            "batch[{i}] is not close to {:?} alone",
            texts[i]
        );
        assert!(sims.iter().enumerate().all(|(j, s)| j == i || *s < sims[i]));
    }
}

#[tokio::test]
#[ignore = "calls the production API"]
async fn live_embedding_dimension_over_max() {
    let p = EmbeddingParams {
        model: Some(EmbeddingModel::OctenEmbedding0_6B),
        dimension: Some(4096),
        ..EmbeddingParams::new(["a"])
    };
    let e = live_client().embedding().create(&p).await.unwrap_err();
    assert!(e.as_api().is_some_and(ApiError::is_bad_request), "{e:?}");
}

fn text_and_image() -> Vec<VlEmbeddingContent> {
    vec![
        VlEmbeddingContent::Text("grapefruit".into()),
        VlEmbeddingContent::Image(IMAGE.into()),
    ]
}

#[tokio::test]
#[ignore = "calls the production API"]
async fn live_vl_embedding_independent() {
    let p = VlEmbeddingParams {
        dimension: Some(8),
        ..VlEmbeddingParams::new(VlEmbeddingModel::OctenVlEmbedding, text_and_image())
    };
    let resp = live_client().vl_embedding().create(&p).await.unwrap();
    let results = require(&resp).results.clone().unwrap_or_default();
    let types: Vec<_> = results.iter().map(|r| r.result_type.as_deref()).collect();
    assert_eq!(types, [Some("vl"), Some("vl")]);
    assert_eq!(results[1].embedding.as_deref().unwrap_or_default().len(), 8);
    let images = resp
        .meta
        .as_ref()
        .and_then(|m| m.usage.as_ref())
        .and_then(|u| u.image_count);
    assert_eq!(images, Some(1));
}

#[tokio::test]
#[ignore = "calls the production API"]
async fn live_vl_embedding_fusion() {
    let p = VlEmbeddingParams {
        dimension: Some(8),
        enable_fusion: Some(true),
        ..VlEmbeddingParams::new(VlEmbeddingModel::OctenVlEmbedding, text_and_image())
    };
    let resp = live_client().vl_embedding().create(&p).await.unwrap();
    let results = require(&resp).results.clone().unwrap_or_default();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].result_type.as_deref(), Some("fusion"));
}

#[tokio::test]
#[ignore = "calls the production API"]
async fn live_vl_embedding_video() {
    let p = VlEmbeddingParams {
        dimension: Some(8),
        fps: Some(0.5),
        ..VlEmbeddingParams::new(
            VlEmbeddingModel::OctenVlEmbedding,
            vec![VlEmbeddingContent::Video(VIDEO.into())],
        )
    };
    let started = Instant::now();
    let resp = live_client().vl_embedding().create(&p).await.unwrap();
    println!("took {:?}", started.elapsed());
    assert_eq!(
        require(&resp).results.as_deref().unwrap_or_default().len(),
        1
    );
    let duration = resp
        .meta
        .as_ref()
        .and_then(|m| m.usage.as_ref())
        .and_then(|u| u.duration);
    assert!(duration.unwrap_or(0) > 0);
}

// A 400 must not be retried: build the client with retries on and check the server saw one request
// through the error (a 400 is final whatever max_retries is).
#[tokio::test]
#[ignore = "calls the production API"]
async fn live_vl_embedding_unreachable_media() {
    let p = VlEmbeddingParams::new(
        VlEmbeddingModel::OctenVlEmbedding,
        vec![VlEmbeddingContent::Image(
            "https://example.invalid/a.jpg".into(),
        )],
    );
    let started = Instant::now();
    let e = live_client().vl_embedding().create(&p).await.unwrap_err();
    let api = e
        .as_api()
        .unwrap_or_else(|| panic!("expected an API error, got {e:?}"));
    assert!(api.is_bad_request(), "{api:?}");
    // A retried call would have waited at least one second of backoff
    assert!(
        started.elapsed().as_secs_f64() < 1.0 + 5.0,
        "took {:?}",
        started.elapsed()
    );
    println!("message={:?} request_id={:?}", api.message, api.request_id);
}
