//! Extract, Embedding and VL Embedding through the public API, against a local mock server.

use octen::*;
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(server: &MockServer) -> Client {
    Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .max_retries(0)
        .build()
        .unwrap()
}

async fn respond(server: &MockServer, p: &str, body: &str) {
    Mock::given(method("POST"))
        .and(path(p))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body.to_owned(), "application/json"))
        .mount(server)
        .await;
}

async fn bodies(server: &MockServer) -> Vec<Value> {
    let reqs = server.received_requests().await.unwrap();
    reqs.iter()
        .map(|r| serde_json::from_slice(&r.body).unwrap())
        .collect()
}

fn field_of(e: Error) -> String {
    match e {
        Error::Validation { field, .. } => field,
        other => panic!("expected a validation error, got {other:?}"),
    }
}

const OK: &str = r#"{"code":0,"request_id":"r","data":{}}"#;

// Go: extract_test.go
#[tokio::test]
async fn extract_urls_validated() {
    let server = MockServer::start().await;
    respond(&server, "/extract", OK).await;
    let c = client(&server);
    let urls = |n: usize| {
        (0..n)
            .map(|i| format!("https://example.com/{i}"))
            .collect::<Vec<_>>()
    };
    // 2048 code points is the limit, counted in characters rather than bytes
    let long_ok = format!("https://example.com/{}", "é".repeat(2048 - 20));
    let too_long = format!("{long_ok}x");
    assert_eq!(long_ok.chars().count(), 2048);
    for (p, field) in [
        (ExtractParams::new(Vec::<String>::new()), "urls"),
        (ExtractParams::new(urls(21)), "urls"),
        (ExtractParams::new(["https://a.com", "  "]), "urls[1]"),
        (ExtractParams::new([""]), "urls[0]"),
        (
            ExtractParams::new(["https://a.com".to_owned(), too_long]),
            "urls[1]",
        ),
    ] {
        assert_eq!(field_of(c.extract().extract(&p).await.unwrap_err()), field);
    }
    assert!(server.received_requests().await.unwrap().is_empty());
    c.extract()
        .extract(&ExtractParams::new(urls(20)))
        .await
        .unwrap();
    c.extract()
        .extract(&ExtractParams::new([long_ok]))
        .await
        .unwrap();
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn extract_sends_documented_fields() {
    let server = MockServer::start().await;
    respond(&server, "/extract", OK).await;
    let c = client(&server);
    c.extract()
        .extract(&ExtractParams::new(["https://a.com"]))
        .await
        .unwrap();
    // An empty links option is sent as {} and asks for links with the server defaults
    let p = ExtractParams {
        include_links: Some(ExtractLinksOptions::default()),
        ..ExtractParams::new(["https://a.com"])
    };
    c.extract().extract(&p).await.unwrap();
    let p = ExtractParams {
        mode: Some(ExtractMode::Auto),
        query: Some("pricing".into()),
        max_age_seconds: Some(300),
        format: Some(Format::Text),
        timeout: Some(60),
        include_images: Some(true),
        include_videos: Some(false),
        include_audio: Some(true),
        include_links: Some(ExtractLinksOptions {
            scope: Some(ExtractLinkScope::PreferExternal),
            max_links: Some(5),
        }),
        ..ExtractParams::new(["https://a.com"])
    };
    c.extract().extract(&p).await.unwrap();
    assert_eq!(
        bodies(&server).await,
        vec![
            json!({"urls": ["https://a.com"]}),
            json!({"urls": ["https://a.com"], "include_links": {}}),
            json!({"urls": ["https://a.com"], "mode": "auto", "query": "pricing", "max_age_seconds": 300, "format": "text",
                   "timeout": 60, "include_images": true, "include_videos": false, "include_audio": true,
                   "include_links": {"scope": "prefer_external", "max_links": 5}}),
        ]
    );
}

#[tokio::test]
async fn extract_decodes_a_partial_failure() {
    let server = MockServer::start().await;
    let body = r##"{"code":0,"request_id":"r","data":{"results":[
        {"url":"https://a.com","status":"success","resolved_mode":"standard","full_content":"# A","page_structure":null,
         "category":{"primary":"news","secondary":null},"links":[{"url":"https://b.com","anchor_text":"B","is_external":true}]},
        {"url":"https://broken.invalid/","status":"failed","error_message":"failed"}]},
        "meta":{"usage":{"total_urls":2,"successful_urls":1,"successful_by_mode":{"standard_urls":1,"advanced_urls":0}}}}"##;
    respond(&server, "/extract", body).await;
    let resp = client(&server)
        .extract()
        .extract(&ExtractParams::new([
            "https://a.com",
            "https://broken.invalid/",
        ]))
        .await
        .unwrap();
    let results = resp.data.as_ref().unwrap().results.as_ref().unwrap();
    assert_eq!(results[0].status.as_deref(), Some("success"));
    assert!(results[0].page_structure.is_none());
    assert_eq!(results[0].category.as_ref().unwrap().secondary, None);
    assert_eq!(
        results[0].links.as_ref().unwrap()[0].is_external,
        Some(true)
    );
    assert_eq!(results[1].error_message.as_deref(), Some("failed"));
    let usage = resp.meta.as_ref().unwrap().usage.as_ref().unwrap();
    assert_eq!(
        usage.successful_by_mode.as_ref().unwrap().standard_urls,
        Some(1)
    );
}

// Go: embedding_test.go
#[tokio::test]
async fn embedding_sends_only_documented_fields() {
    let server = MockServer::start().await;
    respond(&server, "/embedding", OK).await;
    let c = client(&server);
    c.embedding()
        .create(&EmbeddingParams::new(["a", "b"]))
        .await
        .unwrap();
    let p = EmbeddingParams {
        model: Some(EmbeddingModel::OctenEmbedding0_6B),
        dimension: Some(8),
        input_type: Some(InputType::Query),
        ..EmbeddingParams::new(["a"])
    };
    c.embedding().create(&p).await.unwrap();
    // The server answers a dimension above the model maximum with a 400; it is not checked here
    let p = EmbeddingParams {
        dimension: Some(99999),
        ..EmbeddingParams::new(["a"])
    };
    c.embedding().create(&p).await.unwrap();
    assert_eq!(
        bodies(&server).await,
        vec![
            json!({"input": ["a", "b"]}),
            json!({"input": ["a"], "model": "octen-embedding-0.6b", "dimension": 8, "input_type": "query"}),
            json!({"input": ["a"], "dimension": 99999}),
        ]
    );
}

#[tokio::test]
async fn rejects_blank_input_item() {
    let server = MockServer::start().await;
    respond(&server, "/embedding", OK).await;
    let c = client(&server);
    let texts = |n: usize| (0..n).map(|i| format!("t{i}")).collect::<Vec<_>>();
    for (p, field) in [
        (EmbeddingParams::new(Vec::<String>::new()), "input"),
        (EmbeddingParams::new(texts(1001)), "input"),
        // The server accepts a whitespace-only item and bills it (checked 2026-11-26)
        (EmbeddingParams::new(["a", "b", "  "]), "input[2]"),
        (EmbeddingParams::new([""]), "input[0]"),
    ] {
        assert_eq!(field_of(c.embedding().create(&p).await.unwrap_err()), field);
    }
    assert!(server.received_requests().await.unwrap().is_empty());
    c.embedding()
        .create(&EmbeddingParams::new(texts(1000)))
        .await
        .unwrap();
}

// The 200 example of POST /embedding in testdata/openapi.json.
#[tokio::test]
async fn embedding_decodes_response() {
    let server = MockServer::start().await;
    let body = r#"{"code":0,"msg":"success","request_id":"req_abc123def456","data":{"results":[
        {"index":0,"embedding":[0.0123,-0.0456,0.0789]},{"index":1,"embedding":[-0.0011,0.0234,0.0567]}],
        "model":"octen-embedding-0.6b"},"meta":{"usage":{"input_tokens":128},"latency":167,
        "warning":"Input was truncated for one or more items."}}"#;
    respond(&server, "/embedding", body).await;
    let resp = client(&server)
        .embedding()
        .create(&EmbeddingParams::new(["a", "b"]))
        .await
        .unwrap();
    let data = resp.data.as_ref().unwrap();
    let r = &data.results.as_ref().unwrap()[1];
    assert_eq!(
        (r.index, r.embedding.as_ref().unwrap()[2]),
        (Some(1), 0.0567)
    );
    assert_eq!(data.model.as_deref(), Some("octen-embedding-0.6b"));
    let meta = resp.meta.as_ref().unwrap();
    assert_eq!(meta.usage.as_ref().unwrap().input_tokens, Some(128));
    assert_eq!(meta.latency, Some(167.0));
    assert_eq!(
        meta.warning.as_deref(),
        Some("Input was truncated for one or more items.")
    );
}

// Go: vl_embedding_test.go
#[tokio::test]
async fn vl_embedding_sends_nested_contents() {
    let server = MockServer::start().await;
    respond(&server, "/vl-embedding", OK).await;
    let c = client(&server);
    let p = VlEmbeddingParams {
        enable_fusion: Some(true),
        dimension: Some(2048),
        fps: Some(0.3),
        instruct: Some("Represent the product".into()),
        ..VlEmbeddingParams::new(
            VlEmbeddingModel::OctenVlEmbeddingLarge,
            vec![
                VlEmbeddingContent::Text("tent".into()),
                VlEmbeddingContent::Image("https://example.com/a.jpg".into()),
                VlEmbeddingContent::Video("https://example.com/v.mp4".into()),
            ],
        )
    };
    c.vl_embedding().create(&p).await.unwrap();
    let p = VlEmbeddingParams {
        enable_fusion: Some(false),
        ..VlEmbeddingParams::new(
            VlEmbeddingModel::OctenVlEmbedding,
            vec![VlEmbeddingContent::Text("a".into())],
        )
    };
    c.vl_embedding().create(&p).await.unwrap();
    assert_eq!(
        bodies(&server).await,
        vec![
            json!({"model": "octen-vl-embedding-large", "input": {"contents": [{"text": "tent"},
                   {"image": "https://example.com/a.jpg"}, {"video": "https://example.com/v.mp4"}]},
                   "enable_fusion": true, "dimension": 2048, "fps": 0.3, "instruct": "Represent the product"}),
            json!({"model": "octen-vl-embedding", "input": {"contents": [{"text": "a"}]}, "enable_fusion": false}),
        ]
    );
}

#[tokio::test]
async fn vl_embedding_validation() {
    let server = MockServer::start().await;
    respond(&server, "/vl-embedding", OK).await;
    let c = client(&server);
    let with = |contents: Vec<VlEmbeddingContent>| {
        VlEmbeddingParams::new(VlEmbeddingModel::OctenVlEmbedding, contents)
    };
    let texts = |n: usize| {
        (0..n)
            .map(|_| VlEmbeddingContent::Text("a".into()))
            .collect::<Vec<_>>()
    };
    for (p, field) in [
        (with(vec![]), "input.contents"),
        (with(texts(21)), "input.contents"),
        (
            with(vec![
                VlEmbeddingContent::Text("a".into()),
                VlEmbeddingContent::Image(" ".into()),
            ]),
            "input.contents[1]",
        ),
        (
            with(vec![VlEmbeddingContent::Video(String::new())]),
            "input.contents[0]",
        ),
        (
            VlEmbeddingParams {
                fps: Some(f64::NAN),
                ..with(texts(1))
            },
            "fps",
        ),
    ] {
        assert_eq!(
            field_of(c.vl_embedding().create(&p).await.unwrap_err()),
            field
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
    // Counts and ranges are the server's: 6 images, fps 1.5 and a large dimension are sent
    let images = (0..6)
        .map(|_| VlEmbeddingContent::Image("https://example.com/a.jpg".into()))
        .collect();
    let p = VlEmbeddingParams {
        fps: Some(1.5),
        dimension: Some(99999),
        ..with(images)
    };
    c.vl_embedding().create(&p).await.unwrap();
    c.vl_embedding().create(&with(texts(20))).await.unwrap();
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

// The fusion example of POST /vl-embedding, plus a result type and a field this version does not know.
#[tokio::test]
async fn decodes_unknown_fields_and_types() {
    let server = MockServer::start().await;
    let body = r#"{"code":0,"request_id":"a7b8c9d0","data":{"results":[
        {"index":0,"embedding":[0.0156,-0.0298,0.0411],"type":"fusion"},
        {"index":1,"embedding":[0.1],"type":"future","score":0.9}],"model":"octen-vl-embedding-large","extra":true},
        "meta":{"usage":{"input_tokens":6814,"text_tokens":18,"image_tokens":6796,"image_count":2,"duration":22},"warning":""}}"#;
    respond(&server, "/vl-embedding", body).await;
    let p = VlEmbeddingParams::new(
        VlEmbeddingModel::OctenVlEmbedding,
        vec![VlEmbeddingContent::Text("a".into())],
    );
    let resp = client(&server).vl_embedding().create(&p).await.unwrap();
    let results = resp.data.as_ref().unwrap().results.as_ref().unwrap();
    assert_eq!(results[0].result_type.as_deref(), Some("fusion"));
    assert_eq!(results[1].result_type.as_deref(), Some("future"));
    let usage = resp.meta.as_ref().unwrap().usage.as_ref().unwrap();
    assert_eq!((usage.image_count, usage.duration), (Some(2), Some(22)));
    assert_eq!(resp.meta.as_ref().unwrap().warning.as_deref(), Some(""));
    assert_eq!(resp.raw_json(), body.as_bytes());
}
