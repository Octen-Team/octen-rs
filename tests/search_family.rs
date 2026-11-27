//! The ten search endpoints through the public API, against a local mock server.

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

fn ok_body(path: &str) -> String {
    format!(
        r#"{{"code":0,"msg":"success","request_id":"rid{}","data":{{}},"meta":{{"latency":12}}}}"#,
        path.replace('/', "-")
    )
}

async fn mount(server: &MockServer, p: &str) {
    Mock::given(method("POST"))
        .and(path(p))
        .respond_with(ResponseTemplate::new(200).set_body_raw(ok_body(p), "application/json"))
        .mount(server)
        .await;
}

/// The body the server received, as JSON.
async fn last_body(server: &MockServer) -> Value {
    let reqs = server.received_requests().await.unwrap();
    serde_json::from_slice(&reqs.last().expect("a request").body).unwrap()
}

macro_rules! posts_to {
    ($name:ident, $path:literal, |$c:ident| $call:expr, $params:expr) => {
        #[tokio::test]
        async fn $name() {
            let server = MockServer::start().await;
            mount(&server, $path).await;
            let $c = client(&server);
            let resp = $call.await.unwrap();
            assert_eq!(resp.request_id, format!("rid{}", $path.replace('/', "-")));
            assert_eq!(resp.raw_json(), ok_body($path).as_bytes());
            let reqs = server.received_requests().await.unwrap();
            assert_eq!(reqs.len(), 1);
            assert_eq!(reqs[0].url.path(), $path);
            assert_eq!(
                last_body(&server).await,
                serde_json::to_value(&$params).unwrap()
            );
        }
    };
}

posts_to!(
    posts_to_search,
    "/search",
    |c| c.search().search(&SearchParams::new("q")),
    SearchParams::new("q")
);
posts_to!(
    posts_to_search_premier,
    "/search-premier",
    |c| c.search().search_premier(&SearchParams::new("q")),
    SearchParams::new("q")
);
posts_to!(
    posts_to_broad_search,
    "/broad-search",
    |c| c.search().broad_search(&BroadSearchParams::new("q")),
    BroadSearchParams::new("q")
);
posts_to!(
    posts_to_news_search,
    "/news-search",
    |c| c.search().news_search(&NewsSearchParams::new("q")),
    NewsSearchParams::new("q")
);
posts_to!(
    posts_to_business_search,
    "/business-search",
    |c| c.search().business_search(&BusinessSearchParams::new("q")),
    BusinessSearchParams::new("q")
);
posts_to!(
    posts_to_pulse_search,
    "/pulse-search",
    |c| c.search().pulse_search(&PulseSearchParams::new("q")),
    PulseSearchParams::new("q")
);
posts_to!(
    posts_to_place_search,
    "/place-search",
    |c| c.place_search().search(&PlaceSearchParams::new("q")),
    PlaceSearchParams::new("q")
);
posts_to!(
    posts_to_image_search,
    "/image-search",
    |c| c.image_search().search(&ImageSearchParams::text("cats")),
    ImageSearchParams::text("cats")
);
posts_to!(
    posts_to_video_search,
    "/video-search",
    |c| c.video_search().search(&VideoSearchParams::text("knots")),
    VideoSearchParams::text("knots")
);
posts_to!(
    posts_to_design_search,
    "/design-search",
    |c| c
        .design_search()
        .search(&DesignSearchParams::new("pricing page")),
    DesignSearchParams::new("pricing page")
);

#[test]
fn request_bodies_match_the_api_reference() {
    let body = |v: &dyn erased::Ser| v.to_value();
    // Unset fields are left out; empty lists too
    assert_eq!(body(&SearchParams::new("q")), json!({"query": "q"}));
    assert_eq!(
        body(&SearchParams {
            result_type: Some(ResultType::Dynamic),
            language: vec![Language::En, Language::Zh],
            time_range: Some(TimeRange::D),
            ..SearchParams::new("q")
        }),
        json!({"query": "q", "result_type": "dynamic", "language": ["en", "zh"], "time_range": "d"})
    );
    // result_type of Broad Search sits inside search_options
    assert_eq!(
        body(&BroadSearchParams {
            search_options: Some(WebSearchOptions {
                result_type: Some(ResultType::Standard),
                ..Default::default()
            }),
            ..BroadSearchParams::new("q")
        }),
        json!({"query": "q", "search_options": {"result_type": "standard"}})
    );
    assert_eq!(
        body(&ImageSearchParams::text("cats")),
        json!({"inputs": [{"type": "text", "data": "cats"}]})
    );
    assert_eq!(
        body(&ImageSearchParams::image_url("https://example.com/a.jpg")),
        json!({"inputs": [{"type": "image", "url": "https://example.com/a.jpg"}]})
    );
    assert_eq!(
        body(&ImageSearchParams::image_data("aGVsbG8=")),
        json!({"inputs": [{"type": "image", "data": "aGVsbG8="}]})
    );
    assert_eq!(
        body(&VideoSearchParams::text("knots")),
        json!({"inputs": [{"type": "text", "data": "knots"}]})
    );
    assert_eq!(
        body(&DesignSearchParams::group("g1")),
        json!({"images": {"group_id": "g1"}})
    );
    assert_eq!(
        body(&PlaceSearchParams {
            user_location: Some(PlaceUserLocation {
                location_type: Some(UserLocationType::Approximate),
                city: Some("Paris".into()),
                ..Default::default()
            }),
            ..PlaceSearchParams::new("cafe")
        }),
        json!({"query": "cafe", "user_location": {"type": "approximate", "city": "Paris"}})
    );
    // Not HTML-escaped: the body reads like the params
    let raw = serde_json::to_string(&SearchParams::new("<b>&'\"")).unwrap();
    assert_eq!(raw, r#"{"query":"<b>&'\""}"#);
}

mod erased {
    pub trait Ser {
        fn to_value(&self) -> serde_json::Value;
    }
    impl<T: serde::Serialize> Ser for T {
        fn to_value(&self) -> serde_json::Value {
            serde_json::to_value(self).unwrap()
        }
    }
}

fn field_of(e: Error) -> String {
    match e {
        Error::Validation { field, .. } => field,
        other => panic!("expected a validation error, got {other:?}"),
    }
}

// Go: TestBlankQueryRejected and TestValidationSendsNothing
#[tokio::test]
async fn blank_query_rejected_before_sending() {
    let server = MockServer::start().await;
    let c = client(&server);
    for q in ["", " ", "\t\n"] {
        let s = c.search();
        assert_eq!(
            field_of(s.search(&SearchParams::new(q)).await.unwrap_err()),
            "query"
        );
        assert_eq!(
            field_of(s.search_premier(&SearchParams::new(q)).await.unwrap_err()),
            "query"
        );
        assert_eq!(
            field_of(
                s.broad_search(&BroadSearchParams::new(q))
                    .await
                    .unwrap_err()
            ),
            "query"
        );
        assert_eq!(
            field_of(s.news_search(&NewsSearchParams::new(q)).await.unwrap_err()),
            "query"
        );
        assert_eq!(
            field_of(
                s.business_search(&BusinessSearchParams::new(q))
                    .await
                    .unwrap_err()
            ),
            "query"
        );
        assert_eq!(
            field_of(
                s.pulse_search(&PulseSearchParams::new(q))
                    .await
                    .unwrap_err()
            ),
            "query"
        );
        assert_eq!(
            field_of(
                c.place_search()
                    .search(&PlaceSearchParams::new(q))
                    .await
                    .unwrap_err()
            ),
            "query"
        );
        assert_eq!(
            field_of(
                c.design_search()
                    .search(&DesignSearchParams::new(q))
                    .await
                    .unwrap_err()
            ),
            "query"
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

// Go: TestNonFiniteNumbersRejected. serde_json would send null for NaN and infinity.
#[tokio::test]
async fn non_finite_numbers_rejected() {
    let server = MockServer::start().await;
    let c = client(&server);
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let p = PlaceSearchParams {
            latitude: Some(bad),
            ..PlaceSearchParams::new("cafe")
        };
        assert_eq!(
            field_of(c.place_search().search(&p).await.unwrap_err()),
            "latitude"
        );
        let p = PlaceSearchParams {
            latitude: Some(1.0),
            longitude: Some(bad),
            ..PlaceSearchParams::new("cafe")
        };
        assert_eq!(
            field_of(c.place_search().search(&p).await.unwrap_err()),
            "longitude"
        );
    }
    // A blank query is reported first
    let p = PlaceSearchParams {
        latitude: Some(f64::NAN),
        ..PlaceSearchParams::new(" ")
    };
    assert_eq!(
        field_of(c.place_search().search(&p).await.unwrap_err()),
        "query"
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

// Go: TestNewEndpointInputsValidated, the cases a Rust input can express
#[tokio::test]
async fn image_and_video_inputs_validated() {
    let server = MockServer::start().await;
    let c = client(&server);
    let image = |inputs: Vec<ImageSearchInput>| ImageSearchParams {
        inputs,
        ..Default::default()
    };
    let cases = [
        (image(vec![]), "inputs"),
        (
            image(vec![
                ImageSearchInput::Text("a".into()),
                ImageSearchInput::Text("b".into()),
            ]),
            "inputs",
        ),
        (
            image(vec![ImageSearchInput::Text(" \t".into())]),
            "inputs[0].data",
        ),
        (
            image(vec![ImageSearchInput::Text(String::new())]),
            "inputs[0].data",
        ),
        (
            image(vec![ImageSearchInput::ImageUrl(" ".into())]),
            "inputs[0]",
        ),
        (
            image(vec![ImageSearchInput::ImageData(String::new())]),
            "inputs[0]",
        ),
    ];
    for (p, field) in cases {
        assert_eq!(
            field_of(c.image_search().search(&p).await.unwrap_err()),
            field,
            "{p:?}"
        );
    }
    let video = |inputs: Vec<VideoSearchInput>| VideoSearchParams {
        inputs,
        ..Default::default()
    };
    let text = |d: &str| VideoSearchInput::Text { data: d.into() };
    for (p, field) in [
        (video(vec![]), "inputs"),
        (video(vec![text("a"), text("b")]), "inputs"),
        (video(vec![text("\n")]), "inputs[0].data"),
    ] {
        assert_eq!(
            field_of(c.video_search().search(&p).await.unwrap_err()),
            field,
            "{p:?}"
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn design_search_needs_query_or_group() {
    let server = MockServer::start().await;
    mount(&server, "/design-search").await;
    let c = client(&server);
    let none = DesignSearchParams::default();
    assert_eq!(
        field_of(c.design_search().search(&none).await.unwrap_err()),
        "query"
    );
    let blank_group = DesignSearchParams::group("  ");
    assert_eq!(
        field_of(c.design_search().search(&blank_group).await.unwrap_err()),
        "query"
    );
    assert!(server.received_requests().await.unwrap().is_empty());
    // A group alone is enough
    c.design_search()
        .search(&DesignSearchParams::group("g1"))
        .await
        .unwrap();
    assert_eq!(
        last_body(&server).await,
        json!({"images": {"group_id": "g1"}})
    );
}

#[tokio::test]
async fn options_reach_the_request() {
    let server = MockServer::start().await;
    mount(&server, "/search").await;
    let c = client(&server);
    let opts = RequestOptions::new().header("x-trace", "abc");
    c.search()
        .with_options(opts)
        .search(&SearchParams::new("q"))
        .await
        .unwrap();
    let reqs = server.received_requests().await.unwrap();
    assert_eq!(reqs[0].headers.get("x-trace").unwrap(), "abc");
}

#[tokio::test]
async fn decodes_a_full_web_result_and_keeps_raw_json() {
    let server = MockServer::start().await;
    // A UTF-8 BOM is accepted, and kept in the raw body
    let body = "\u{feff}".to_owned()
        + r#"{"code":0,"request_id":"r1","data":{"query":"q","results":[{"title":null,"url":"https://a.com","highlight":"h",
        "cover_image":{"url":"https://a.com/c.jpg"},"images":[{"url":"https://a.com/i.jpg","description":"d"}],"future_field":1}]},
        "meta":{"usage":{"num_search_queries":1},"result_type":"dynamic","latency":120.5,"warning":null}}"#;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body.clone(), "application/json"))
        .mount(&server)
        .await;
    let resp = client(&server)
        .search()
        .search(&SearchParams::new("q"))
        .await
        .unwrap();
    assert_eq!(resp.raw_json(), body.as_bytes());
    let data = resp.data.as_ref().unwrap();
    let r = &data.results.as_ref().unwrap()[0];
    assert_eq!(
        (r.title.as_deref(), r.url.as_deref()),
        (None, Some("https://a.com"))
    );
    assert_eq!(
        r.images.as_ref().unwrap()[0].description.as_deref(),
        Some("d")
    );
    let meta = resp.meta.as_ref().unwrap();
    assert_eq!(meta.latency, Some(120.5));
    assert_eq!(meta.result_type.as_deref(), Some("dynamic"));
    assert_eq!(meta.usage.as_ref().unwrap().num_search_queries, Some(1));
}

#[test]
fn business_entities_decode_by_type() {
    let json = json!({"code": 0, "request_id": "r", "data": {"entities": [
        {"type": "company", "name": "Acme", "identifiers": {"stock_ticker": "ACME"}, "metrics": {"stock": {"price": 12.5}}},
        {"type": "person", "name": "Ada", "career": [{"organization": "Acme", "title": "CTO"}]},
        {"type": "fund", "name": "Some Fund"},
        {"name": "no type"}
    ]}});
    let resp: BusinessSearchResponse = serde_json::from_value(json.clone()).unwrap();
    let entities = resp.data.as_ref().unwrap().entities.as_ref().unwrap();
    match &entities[0] {
        BusinessEntity::Company(c) => {
            assert_eq!(c.name.as_deref(), Some("Acme"));
            assert_eq!(
                c.metrics.as_ref().unwrap().stock.as_ref().unwrap().price,
                Some(12.5)
            );
        }
        other => panic!("expected a company, got {other:?}"),
    }
    match &entities[1] {
        BusinessEntity::Person(p) => {
            assert_eq!(p.career.as_ref().unwrap()[0].title.as_deref(), Some("CTO"))
        }
        other => panic!("expected a person, got {other:?}"),
    }
    // An unknown or missing type keeps the entity's JSON instead of failing
    match &entities[2] {
        BusinessEntity::Other { entity_type, raw } => {
            assert_eq!(entity_type.as_deref(), Some("fund"));
            assert_eq!(raw["name"], "Some Fund");
        }
        other => panic!("expected other, got {other:?}"),
    }
    assert!(matches!(
        &entities[3],
        BusinessEntity::Other {
            entity_type: None,
            ..
        }
    ));
    // Encoding writes the type back, so a stored response decodes again
    let again = serde_json::to_value(&resp).unwrap();
    assert_eq!(again["data"]["entities"], json["data"]["entities"]);
}

#[tokio::test]
async fn api_errors_pass_through() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(403)
                .set_body_raw(r#"{"code":403,"msg":"no access"}"#, "application/json"),
        )
        .mount(&server)
        .await;
    let e = client(&server)
        .video_search()
        .search(&VideoSearchParams::text("knots"))
        .await
        .unwrap_err();
    let api = e.as_api().expect("an API error");
    assert!(api.is_permission_denied());
    assert_eq!(api.message, "no access");
}
