//! The request and response types against the OpenAPI spec: testdata/openapi.json, or the file in
//! OCTEN_SPEC_PATH (the weekly drift check points it at the live spec).

use std::collections::{BTreeMap, BTreeSet};

use octen::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};

fn spec() -> Value {
    let path = std::env::var("OCTEN_SPEC_PATH").unwrap_or_else(|_| "testdata/openapi.json".into());
    serde_json::from_str(
        &std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}")),
    )
    .unwrap()
}

fn known_diffs() -> BTreeSet<String> {
    let v: Vec<Value> = serde_json::from_str(include_str!("../testdata/known_diffs.json")).unwrap();
    v.iter()
        .map(|d| d["path"].as_str().unwrap().to_owned())
        .collect()
}

/// A schema with `$ref` followed and `allOf` members merged.
fn resolve(spec: &Value, schema: &Value) -> Value {
    if let Some(r) = schema.get("$ref").and_then(Value::as_str) {
        let name = r.trim_start_matches("#/components/schemas/");
        return resolve(spec, &spec["components"]["schemas"][name]);
    }
    if let Some(all) = schema.get("allOf").and_then(Value::as_array) {
        // The schema's own properties count too, alongside its allOf members
        let mut merged = json!({"type": "object", "properties": schema.get("properties").cloned().unwrap_or(json!({}))});
        for member in all {
            let m = resolve(spec, member);
            if let Some(props) = m.get("properties").and_then(Value::as_object) {
                for (k, v) in props {
                    merged["properties"][k] = v.clone();
                }
            }
        }
        return merged;
    }
    schema.clone()
}

fn ref_name(schema: &Value) -> String {
    schema["$ref"]
        .as_str()
        .unwrap()
        .trim_start_matches("#/components/schemas/")
        .to_owned()
}

fn request_schema(spec: &Value, path: &str) -> Value {
    spec["paths"][path]["post"]["requestBody"]["content"]["application/json"]["schema"].clone()
}

/// Compares the keys of `value` with the schema's properties, level by level; `at` names the
/// position like the entries of known_diffs.json.
fn compare(spec: &Value, schema: &Value, value: &Value, at: &str, diffs: &mut Vec<String>) {
    let schema = resolve(spec, schema);
    match value {
        Value::Object(obj) => {
            let Some(props) = schema.get("properties").and_then(Value::as_object) else {
                return;
            };
            let sdk: BTreeSet<&String> = obj.keys().collect();
            let spec_keys: BTreeSet<&String> = props.keys().collect();
            for k in sdk.difference(&spec_keys) {
                diffs.push(format!("{at}.{k}: in SDK but not in spec"));
            }
            for k in spec_keys.difference(&sdk) {
                diffs.push(format!("{at}.{k}: in spec but not set by the SDK"));
            }
            for (k, v) in obj {
                if let Some(p) = props.get(k) {
                    compare(spec, p, v, &format!("{at}.{k}"), diffs);
                }
            }
        }
        Value::Array(items) => {
            // The union of the keys of every element, so variants with different keys are all covered
            let mut union = serde_json::Map::new();
            for item in items {
                if let Value::Object(o) = item {
                    union.extend(o.clone());
                }
            }
            if !union.is_empty() {
                compare(spec, &schema["items"], &Value::Object(union), at, diffs);
            }
        }
        _ => {}
    }
}

/// Every request field set, so each property of the spec must appear in the serialized body.
fn full_requests() -> Vec<(&'static str, Value)> {
    let s = |v: &str| Some(v.to_owned());
    let strings = |v: &str| vec![v.to_owned()];
    let highlight = Some(HighlightOptions {
        enable: Some(true),
        max_tokens: Some(100),
    });
    let full_content = Some(FullContentOptions {
        enable: Some(true),
        max_tokens: Some(200),
    });
    let web = SearchParams {
        query: "q".into(),
        result_type: Some(ResultType::Dynamic),
        count: Some(5),
        include_domains: strings("a.com"),
        exclude_domains: strings("b.com"),
        include_text: strings("x"),
        exclude_text: strings("y"),
        time_basis: Some(TimeBasis::Published),
        time_range: Some(TimeRange::Week),
        start_time: s("2026-01-01"),
        end_time: s("2026-02-01"),
        language: vec![Language::En],
        highlight: highlight.clone(),
        full_content: full_content.clone(),
        format: Some(Format::Text),
        safesearch: Some(Safesearch::Strict),
        include_images: Some(true),
    };
    let broad = BroadSearchParams {
        query: "q".into(),
        max_queries: Some(3),
        search_options: Some(WebSearchOptions {
            result_type: Some(ResultType::Standard),
            count: Some(5),
            include_domains: strings("a.com"),
            exclude_domains: strings("b.com"),
            include_text: strings("x"),
            exclude_text: strings("y"),
            time_basis: Some(TimeBasis::Auto),
            time_range: Some(TimeRange::Day),
            start_time: s("2026-01-01"),
            end_time: s("2026-02-01"),
            language: vec![Language::De],
            highlight: highlight.clone(),
            full_content: full_content.clone(),
            format: Some(Format::Markdown),
            safesearch: Some(Safesearch::Off),
            include_images: Some(false),
        }),
    };
    let news = NewsSearchParams {
        query: "q".into(),
        count: Some(5),
        include_domains: strings("a.com"),
        exclude_domains: strings("b.com"),
        time_basis: Some(TimeBasis::Crawled),
        time_range: Some(TimeRange::M),
        start_time: s("2026-01-01"),
        end_time: s("2026-02-01"),
        language: vec![Language::Ja],
        highlight: highlight.clone(),
        full_content: full_content.clone(),
        subjects: Some(NewsSubjectOptions {
            enable: Some(true),
            count: Some(3),
            max_sub_news: Some(2),
        }),
    };
    let business = BusinessSearchParams {
        query: "q".into(),
        count: Some(5),
        include_domains: strings("a.com"),
        exclude_domains: strings("b.com"),
        time_basis: Some(TimeBasis::Published),
        time_range: Some(TimeRange::Year),
        start_time: s("2026-01-01"),
        end_time: s("2026-02-01"),
        language: vec![Language::Zh],
        highlight: highlight.clone(),
        full_content: full_content.clone(),
        entities: Some(BusinessEntityOptions {
            enable: Some(true),
            count: Some(2),
            max_activities: Some(3),
            max_news: Some(4),
        }),
    };
    let pulse = PulseSearchParams {
        query: "q".into(),
        mode: Some(PulseMode::Lite),
        count: Some(5),
        highlight: highlight.clone(),
        full_content: full_content.clone(),
        safesearch: Some(Safesearch::Strict),
        include_images: Some(true),
    };
    let place = PlaceSearchParams {
        query: "q".into(),
        places: Some(PlaceCardOptions {
            count: Some(3),
            max_reviews: Some(2),
        }),
        web_search: Some(PlaceWebSearchOptions {
            enable: Some(true),
            count: Some(3),
            highlight: highlight.clone(),
            full_content: full_content.clone(),
            include_domains: strings("a.com"),
            exclude_domains: strings("b.com"),
            language: vec![Language::Fr],
            time_basis: Some(TimeBasis::Auto),
            time_range: Some(TimeRange::W),
            start_time: s("2026-01-01"),
            end_time: s("2026-02-01"),
        }),
        user_location: Some(PlaceUserLocation {
            location_type: Some(UserLocationType::Approximate),
            city: s("Paris"),
            region: s("IDF"),
            country: s("FR"),
            timezone: s("Europe/Paris"),
        }),
        latitude: Some(48.85),
        longitude: Some(2.35),
        radius: Some(1000),
    };
    let image = ImageSearchParams {
        // A text input and an image input, so the keys of both variants are compared
        inputs: vec![
            ImageSearchInput::Text("q".into()),
            ImageSearchInput::ImageUrl("https://e.com/a.jpg".into()),
        ],
        count: Some(5),
        include_domains: strings("a.com"),
        exclude_domains: strings("b.com"),
        safesearch: Some(Safesearch::Off),
    };
    let video = VideoSearchParams {
        count: Some(5),
        time_range: Some(TimeRange::Month),
        start_time: s("2026-01-01"),
        end_time: s("2026-02-01"),
        safesearch: Some(Safesearch::Strict),
        ..VideoSearchParams::text("q")
    };
    let design = DesignSearchParams {
        query: s("pricing page"),
        images: Some(DesignImageOptions {
            enable: Some(true),
            count: Some(4),
            design_facts: Some(true),
            snippet: Some(DesignSnippetOptions {
                enable: Some(true),
                max_tokens: Some(300),
            }),
            group_id: s("g1"),
        }),
        web_search: Some(DesignWebSearchOptions {
            enable: Some(true),
            count: Some(3),
            highlight,
            full_content,
            include_domains: strings("a.com"),
            exclude_domains: strings("b.com"),
            language: vec![Language::Es],
            time_basis: Some(TimeBasis::Published),
            time_range: Some(TimeRange::Y),
            start_time: s("2026-01-01"),
            end_time: s("2026-02-01"),
        }),
    };
    let extract = ExtractParams {
        mode: Some(ExtractMode::Advanced),
        query: s("pricing"),
        max_age_seconds: Some(300),
        format: Some(Format::Markdown),
        timeout: Some(30),
        include_images: Some(true),
        include_videos: Some(true),
        include_audio: Some(true),
        include_links: Some(ExtractLinksOptions {
            scope: Some(ExtractLinkScope::PreferInternal),
            max_links: Some(10),
        }),
        ..ExtractParams::new(["https://a.com"])
    };
    let embedding = EmbeddingParams {
        model: Some(EmbeddingModel::OctenEmbedding4B),
        dimension: Some(8),
        input_type: Some(InputType::Document),
        ..EmbeddingParams::new(["a"])
    };
    let vl = VlEmbeddingParams {
        enable_fusion: Some(true),
        dimension: Some(8),
        fps: Some(0.5),
        instruct: s("Represent the image"),
        ..VlEmbeddingParams::new(
            VlEmbeddingModel::OctenVlEmbedding,
            vec![
                VlEmbeddingContent::Text("a".into()),
                VlEmbeddingContent::Image("https://e.com/a.jpg".into()),
                VlEmbeddingContent::Video("https://e.com/v.mp4".into()),
            ],
        )
    };
    let v = |p: &dyn erased::Ser| p.to_value();
    vec![
        ("/search", v(&web)),
        ("/search-premier", v(&web)),
        ("/broad-search", v(&broad)),
        ("/news-search", v(&news)),
        ("/business-search", v(&business)),
        ("/pulse-search", v(&pulse)),
        ("/place-search", v(&place)),
        ("/image-search", v(&image)),
        ("/video-search", v(&video)),
        ("/design-search", v(&design)),
        ("/extract", v(&extract)),
        ("/embedding", v(&embedding)),
        ("/vl-embedding", v(&vl)),
    ]
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

#[test]
fn request_fields_match_spec() {
    let spec = spec();
    let known = known_diffs();
    let mut diffs = Vec::new();
    for (path, body) in full_requests() {
        let schema = request_schema(&spec, path);
        compare(&spec, &schema, &body, &ref_name(&schema), &mut diffs);
    }
    let unexpected: Vec<_> = diffs
        .iter()
        .filter(|d| !known.iter().any(|k| d.starts_with(&format!("{k}:"))))
        .collect();
    assert!(
        unexpected.is_empty(),
        "request fields differ from the spec:\n{unexpected:#?}"
    );
}

/// The values of the property at `path` (dots between levels) of a request schema, from `enum` or `items.enum`.
fn spec_enum(spec: &Value, schema: &str, path: &str) -> Vec<String> {
    let mut node = resolve(spec, &spec["components"]["schemas"][schema]);
    for key in path.split('.') {
        node = resolve(spec, &node["properties"][key]);
        if node.get("type") == Some(&json!("array")) {
            node = resolve(spec, &node["items"]);
        }
    }
    let values = node["enum"]
        .as_array()
        .unwrap_or_else(|| panic!("{schema}.{path} has no enum"));
    values
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect()
}

fn strs<T: std::fmt::Display>(all: &[T]) -> Vec<String> {
    all.iter().map(ToString::to_string).collect()
}

#[test]
fn enum_values_match_spec() {
    let spec = spec();
    let cases: Vec<(&str, &str, Vec<String>)> = vec![
        ("SearchRequest", "result_type", strs(ResultType::ALL)),
        ("SearchRequest", "time_basis", strs(TimeBasis::ALL)),
        ("SearchRequest", "time_range", strs(TimeRange::ALL)),
        ("SearchRequest", "language", strs(Language::ALL)),
        ("SearchRequest", "format", strs(Format::ALL)),
        ("SearchRequest", "safesearch", strs(Safesearch::ALL)),
        (
            "BroadSearchRequest",
            "search_options.result_type",
            strs(ResultType::ALL),
        ),
        (
            "BroadSearchRequest",
            "search_options.time_basis",
            strs(TimeBasis::ALL),
        ),
        (
            "BroadSearchRequest",
            "search_options.time_range",
            strs(TimeRange::ALL),
        ),
        (
            "BroadSearchRequest",
            "search_options.language",
            strs(Language::ALL),
        ),
        (
            "BroadSearchRequest",
            "search_options.format",
            strs(Format::ALL),
        ),
        (
            "BroadSearchRequest",
            "search_options.safesearch",
            strs(Safesearch::ALL),
        ),
        ("NewsSearchRequest", "time_basis", strs(TimeBasis::ALL)),
        ("NewsSearchRequest", "time_range", strs(TimeRange::ALL)),
        ("NewsSearchRequest", "language", strs(Language::ALL)),
        ("BusinessSearchRequest", "time_basis", strs(TimeBasis::ALL)),
        ("BusinessSearchRequest", "time_range", strs(TimeRange::ALL)),
        ("BusinessSearchRequest", "language", strs(Language::ALL)),
        ("PulseSearchRequest", "mode", strs(PulseMode::ALL)),
        ("PulseSearchRequest", "safesearch", strs(Safesearch::ALL)),
        (
            "PlaceSearchRequest",
            "web_search.language",
            strs(Language::ALL),
        ),
        (
            "PlaceSearchRequest",
            "web_search.time_basis",
            strs(TimeBasis::ALL),
        ),
        (
            "PlaceSearchRequest",
            "web_search.time_range",
            strs(TimeRange::ALL),
        ),
        (
            "PlaceSearchRequest",
            "user_location.type",
            strs(UserLocationType::ALL),
        ),
        (
            "ImageSearchRequest",
            "inputs.type",
            vec!["text".into(), "image".into()],
        ),
        ("ImageSearchRequest", "safesearch", strs(Safesearch::ALL)),
        ("VideoSearchRequest", "inputs.type", vec!["text".into()]),
        ("VideoSearchRequest", "time_range", strs(TimeRange::ALL)),
        ("VideoSearchRequest", "safesearch", strs(Safesearch::ALL)),
        (
            "DesignSearchRequest",
            "web_search.language",
            strs(Language::ALL),
        ),
        (
            "DesignSearchRequest",
            "web_search.time_basis",
            strs(TimeBasis::ALL),
        ),
        (
            "DesignSearchRequest",
            "web_search.time_range",
            strs(TimeRange::ALL),
        ),
        ("ExtractRequest", "mode", strs(ExtractMode::ALL)),
        ("ExtractRequest", "format", strs(Format::ALL)),
        (
            "ExtractRequest",
            "include_links.scope",
            strs(ExtractLinkScope::ALL),
        ),
        ("EmbeddingRequest", "model", strs(EmbeddingModel::ALL)),
        ("EmbeddingRequest", "input_type", strs(InputType::ALL)),
        ("VLEmbeddingRequest", "model", strs(VlEmbeddingModel::ALL)),
    ];
    let mut diffs = Vec::new();
    for (schema, path, sdk) in cases {
        let spec_values = spec_enum(&spec, schema, path);
        if spec_values != sdk {
            diffs.push(format!(
                "{schema}.{path}: spec {spec_values:?}, SDK {sdk:?}"
            ));
        }
    }
    assert!(
        diffs.is_empty(),
        "enum values differ from the spec:\n{diffs:#?}"
    );
}

/// Decodes the example into `T` and encodes it back: an error, or the example's keys the type dropped.
fn roundtrip<T: DeserializeOwned + Serialize>(example: &Value) -> Result<Vec<String>, String> {
    let decoded: T = serde_json::from_value(example.clone()).map_err(|e| e.to_string())?;
    let encoded = serde_json::to_value(&decoded).unwrap();
    let mut lost = Vec::new();
    lost_keys(example, &encoded, "", &mut lost);
    Ok(lost)
}

fn lost_keys(example: &Value, encoded: &Value, at: &str, lost: &mut Vec<String>) {
    match (example, encoded) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in a {
                if v.is_null() {
                    continue;
                }
                match b.get(k) {
                    None => lost.push(format!("{at}.{k}")),
                    Some(w) => lost_keys(v, w, &format!("{at}.{k}"), lost),
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            for (i, (v, w)) in a.iter().zip(b).enumerate() {
                lost_keys(v, w, &format!("{at}[{i}]"), lost);
            }
        }
        _ => {}
    }
}

type Check = fn(&Value) -> Result<Vec<String>, String>;

/// Every public method: its path and the check of its response examples.
fn contract_table() -> BTreeMap<&'static str, (&'static str, Check)> {
    BTreeMap::from([
        (
            "search().search",
            ("/search", roundtrip::<SearchResponse> as Check),
        ),
        (
            "search().search_premier",
            ("/search-premier", roundtrip::<SearchResponse>),
        ),
        (
            "search().broad_search",
            ("/broad-search", roundtrip::<BroadSearchResponse>),
        ),
        (
            "search().news_search",
            ("/news-search", roundtrip::<NewsSearchResponse>),
        ),
        (
            "search().business_search",
            ("/business-search", roundtrip::<BusinessSearchResponse>),
        ),
        (
            "search().pulse_search",
            ("/pulse-search", roundtrip::<PulseSearchResponse>),
        ),
        (
            "place_search().search",
            ("/place-search", roundtrip::<PlaceSearchResponse>),
        ),
        (
            "image_search().search",
            ("/image-search", roundtrip::<ImageSearchResponse>),
        ),
        (
            "video_search().search",
            ("/video-search", roundtrip::<VideoSearchResponse>),
        ),
        (
            "design_search().search",
            ("/design-search", roundtrip::<DesignSearchResponse>),
        ),
        (
            "extract().extract",
            ("/extract", roundtrip::<ExtractResponse>),
        ),
        (
            "embedding().create",
            ("/embedding", roundtrip::<EmbeddingResponse>),
        ),
        (
            "vl_embedding().create",
            ("/vl-embedding", roundtrip::<VlEmbeddingResponse>),
        ),
    ])
}

fn examples(spec: &Value, path: &str) -> Vec<(String, Value)> {
    let content = &spec["paths"][path]["post"]["responses"]["200"]["content"]["application/json"];
    let mut out = Vec::new();
    if let Some(ex) = content.get("example") {
        out.push(("example".to_owned(), ex.clone()));
    }
    if let Some(map) = content.get("examples").and_then(Value::as_object) {
        for (name, ex) in map {
            out.push((name.clone(), ex["value"].clone()));
        }
    }
    out
}

#[test]
fn spec_examples_decode() {
    let spec = spec();
    let mut problems = Vec::new();
    for (method, (path, check)) in contract_table() {
        let exs = examples(&spec, path);
        if exs.is_empty() {
            problems.push(format!("{method} ({path}): the spec has no 200 example"));
        }
        for (name, ex) in exs {
            match check(&ex) {
                Err(e) => problems.push(format!("{path} example {name}: does not decode: {e}")),
                Ok(lost) if !lost.is_empty() => problems.push(format!(
                    "{path} example {name}: fields not modelled: {lost:?}"
                )),
                Ok(_) => {}
            }
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

// A new method must be added to the contract table (and so to these checks); the list below is
// the public surface, kept by hand like Go's TestContractCoversEverySDKMethod.
#[test]
fn contract_table_covers_every_method() {
    let methods = [
        "search().search",
        "search().search_premier",
        "search().broad_search",
        "search().news_search",
        "search().business_search",
        "search().pulse_search",
        "place_search().search",
        "image_search().search",
        "video_search().search",
        "design_search().search",
        "extract().extract",
        "embedding().create",
        "vl_embedding().create",
    ];
    let table = contract_table();
    let listed: BTreeSet<&str> = methods.into_iter().collect();
    let covered: BTreeSet<&str> = table.keys().copied().collect();
    assert_eq!(listed, covered);
    let paths: BTreeSet<&str> = table.values().map(|(p, _)| *p).collect();
    let requests: BTreeSet<&str> = full_requests().into_iter().map(|(p, _)| p).collect();
    assert_eq!(
        paths, requests,
        "every method needs a full request in request_fields_match_spec"
    );
}
