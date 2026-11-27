# octen

Official Rust SDK for the [Octen](https://docs.octen.ai) API. It is asynchronous (Tokio), and needs Rust 1.85 or newer.

It covers thirteen endpoints: Web Search, Web Search Premier, Broad Search, News Search, Business Search, Pulse Search, Place Search, Image Search, Video Search (invite-only beta), Design Search, Extract, Embedding and VL Embedding.

- [Install](#install)
- [Quick start](#quick-start)
- [Methods](#methods)
- [Client options](#client-options)
- [Local validation](#local-validation)
- [Errors](#errors)
- [Retries and timeouts](#retries-and-timeouts)
- [Differences from the other SDKs](#differences-from-the-other-sdks)
- [Versioning](#versioning)
- [Development](#development)

## Install

```sh
cargo add octen
cargo add tokio --features macros,rt-multi-thread
```

## Quick start

```rust,no_run
use octen::{Client, SearchParams};

#[tokio::main]
async fn main() -> Result<(), octen::Error> {
    let client = Client::from_env()?; // reads OCTEN_API_KEY; or Client::new("sk-...")
    let resp = client.search().search(&SearchParams::new("rust async runtimes")).await?;
    for r in resp.data.and_then(|d| d.results).unwrap_or_default() {
        println!("{} {}", r.title.unwrap_or_default(), r.url.unwrap_or_default());
    }
    Ok(())
}
```

Request structs have public fields, `Default`, and a constructor for the required input; set the rest with struct update syntax. Every response field is an `Option`: the server may leave a field out, and new fields never break decoding. `raw_json()` returns the body exactly as received, including fields this SDK does not model, and `request_id` identifies the call when you contact support.

## Methods

| Method | Endpoint |
|---|---|
| `client.search().search(&SearchParams)` | `POST /search` |
| `client.search().search_premier(&SearchParams)` | `POST /search-premier` |
| `client.search().broad_search(&BroadSearchParams)` | `POST /broad-search` |
| `client.search().news_search(&NewsSearchParams)` | `POST /news-search` |
| `client.search().business_search(&BusinessSearchParams)` | `POST /business-search` |
| `client.search().pulse_search(&PulseSearchParams)` | `POST /pulse-search` |
| `client.place_search().search(&PlaceSearchParams)` | `POST /place-search` |
| `client.image_search().search(&ImageSearchParams)` | `POST /image-search` |
| `client.video_search().search(&VideoSearchParams)` | `POST /video-search` |
| `client.design_search().search(&DesignSearchParams)` | `POST /design-search` |
| `client.extract().extract(&ExtractParams)` | `POST /extract` |
| `client.embedding().create(&EmbeddingParams)` | `POST /embedding` |
| `client.vl_embedding().create(&VlEmbeddingParams)` | `POST /vl-embedding` |

```rust,no_run
use octen::*;

# async fn run(client: Client) -> Result<(), Error> {
// Web Search, with filters
let web = client.search().search(&SearchParams {
    count: Some(5),
    result_type: Some(ResultType::Dynamic), // only relevant results, up to count
    time_range: Some(TimeRange::Month),
    include_domains: vec!["docs.rs".into()],
    language: vec![Language::En],
    ..SearchParams::new("tokio select macro")
}).await?;

// Broad Search: the query expands into several searches (default timeout 120 s)
let broad = client.search().broad_search(&BroadSearchParams {
    max_queries: Some(3),
    search_options: Some(WebSearchOptions { count: Some(5), ..Default::default() }),
    ..BroadSearchParams::new("vector database benchmarks")
}).await?;

// News, Business and Pulse Search take a query in the same way
let news = client.search().news_search(&NewsSearchParams::new("chip export rules")).await?;
let business = client.search().business_search(&BusinessSearchParams::new("Octen")).await?;
for entity in business.data.and_then(|d| d.entities).unwrap_or_default() {
    match entity {
        BusinessEntity::Company(c) => println!("company {:?}", c.name),
        BusinessEntity::Person(p) => println!("person {:?}", p.name),
        other => println!("another kind of entity: {other:?}"), // keep a fallback arm
    }
}

// Place Search around a point
let places = client.place_search().search(&PlaceSearchParams {
    latitude: Some(35.6812),
    longitude: Some(139.7671),
    radius: Some(1000),
    ..PlaceSearchParams::new("coffee")
}).await?;

// Image Search by text, by an image URL or by base64 image data
let images = client.image_search().search(&ImageSearchParams::image_url("https://example.com/cat.jpg")).await?;

// Video Search (invite-only beta) and Design Search
let videos = client.video_search().search(&VideoSearchParams::text("how to tie a bowline knot")).await?;
let designs = client.design_search().search(&DesignSearchParams::new("SaaS pricing page")).await?;
let deck = client.design_search().search(&DesignSearchParams::group("a group_id from a result")).await?;

// Extract: one result per URL; match results to your input by url
let pages = client.extract().extract(&ExtractParams {
    format: Some(Format::Markdown),
    include_links: Some(ExtractLinksOptions::default()), // sent as {}: links with the server defaults
    ..ExtractParams::new(["https://octen.ai/", "https://docs.octen.ai/"])
}).await?;
for item in pages.data.and_then(|d| d.results).unwrap_or_default() {
    match item.status.as_deref() {
        Some("success") => println!("{:?}: {} chars", item.url, item.full_content.unwrap_or_default().len()),
        _ => println!("{:?} failed: {:?}", item.url, item.error_message), // not billed
    }
}

// Embedding: one vector per text, matched by index
let emb = client.embedding().create(&EmbeddingParams {
    model: Some(EmbeddingModel::OctenEmbedding4B),
    dimension: Some(1024),
    input_type: Some(InputType::Query), // Document, or unset, for the texts you index
    ..EmbeddingParams::new(["What is vector search?"])
}).await?;

// VL Embedding: text, images and video in one vector space
let vl = client.vl_embedding().create(&VlEmbeddingParams {
    enable_fusion: Some(true), // one fused vector instead of one per item
    ..VlEmbeddingParams::new(
        VlEmbeddingModel::OctenVlEmbedding,
        vec![
            VlEmbeddingContent::Text("Outdoor tent, 3-4 person".into()),
            VlEmbeddingContent::Image("https://example.com/tent.jpg".into()), // a public URL or base64 data
        ],
    )
}).await?;
# let _ = (web, broad, news, places, images, videos, designs, deck, emb, vl);
# Ok(()) }
```

Notes on individual endpoints:

- **Result type.** `result_type` on Web Search and Web Search Premier, and inside Broad Search's `search_options`, is `Standard` (the default) or `Dynamic`, which returns only relevant results up to `count` and adjusts the highlight length per result. Web Search echoes the type used in `meta.result_type`.
- **Video Search** is an invite-only beta; a key without access gets a 403.
- **Design Search** needs a query unless `images.group_id` is set; `DesignSearchParams::group` returns a whole page or deck in order. `design_facts` and `group.master` are maps decided by the server.
- **Business Search** entities decode by `type` into `Company` or `Person`; any other or missing type decodes as `Other` with the entity's JSON, so new types never break decoding.
- **Extract.** A call can succeed while some URLs fail (`status` is `failed` with an `error_message`); failed URLs are not billed. With `query`, each result carries `highlights` instead of `full_content`. `mode` is `Standard` (the default), `Advanced` (slower, more expensive, for pages that are hard to reach) or `Auto`; `resolved_mode` is the mode a URL is billed at, and `meta.usage.successful_by_mode` counts them. The body's `timeout` (1 to 60 s, 30 by default) bounds each URL on the server; it is not the HTTP timeout, which the SDK derives from it (see [Retries and timeouts](#retries-and-timeouts)).
- **Embedding.** `results[i].index` is the position in `input`. Texts longer than 10,000 characters are cut by the server, which says so in `meta.warning`.
- **VL Embedding.** The server must be able to fetch an image or video URL; when it cannot, it answers with a 400 (`Invalid params.` or `Video is not reachable`). A long video can take longer than the default 30 s: set a timeout for that call. Each result's `result_type` is `vl` or `fusion`, returned as sent.

## Client options

```rust,no_run
use std::time::Duration;
use octen::{Client, RequestOptions, SearchParams};

# async fn run() -> Result<(), octen::Error> {
let client = Client::builder()
    .api_key("sk-...")                   // default: $OCTEN_API_KEY; build fails if neither is set
    .base_url("https://api.octen.ai")    // default shown
    .timeout(Duration::from_secs(60))    // per attempt; applies to every endpoint once set
    .max_retries(3)                      // default 3 (at most 4 requests)
    .header("x-trace-id", "abc")         // sent with every request; must not be x-api-key
    .build()?;

// Per call: a timeout or headers for the calls made through this handle
let opts = RequestOptions::new().timeout(Duration::from_secs(120)).header("x-trace-id", "def");
client.search().with_options(opts).search(&SearchParams::new("hello")).await?;
# Ok(()) }
```

- `Client` is cheap to clone; clones share one connection pool.
- The API key is trimmed and sent only in the `x-api-key` header. The `User-Agent` is `octen-rust-sdk/<version>`.
- Redirects are not followed, so the key cannot be forwarded to another host; a redirect is reported as an `Error::Api` saying so.
- Proxies come from the `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY` and `NO_PROXY` environment variables. Operating-system proxy settings are not read.
- `http_client(reqwest::Client)` sends requests through your own client, for custom TLS, proxies or pooling. Its own timeout applies on top of the SDK's, and its redirect policy is kept, so configure it not to follow redirects.
- Invalid options (no key, a base URL that is not an absolute http(s) URL or that has credentials, a query or a fragment, a zero timeout, an invalid header name, a header value or key with a control character other than tab) make `build()` return an `Error::Validation` whose `field` names the option. A bad per-call option is returned by the call the same way.

## Local validation

Before sending anything the SDK checks only the required inputs, and reports a problem as `Error::Validation` with the offending `field`:

- `query` must not be blank on the endpoints that take one; Design Search needs a non-blank `query` or `images.group_id`.
- Image Search and Video Search need exactly one input in `inputs` (`inputs`), and its value must not be blank (`inputs[0].data`, or `inputs[0]` for an image).
- Extract needs 1 to 20 URLs (`urls`), each non-blank and at most 2048 characters, counted as Unicode code points (`urls[1]` names the offending one).
- Embedding needs 1 to 1000 texts (`input`), none blank (`input[2]`), because the server accepts and bills a blank text.
- VL Embedding needs 1 to 20 items (`input.contents`), none blank (`input.contents[0]`).
- `latitude`, `longitude` and `fps` must be finite: JSON has no NaN or infinity.

Value ranges (such as `count`, `dimension` or the number of images) are left to the server, which answers out-of-range values with a 400. Enum fields are typed, so only documented values can be sent.

## Errors

| Situation | Error |
|---|---|
| A non-2xx response, or a 2xx whose body is not a usable result | `Error::Api(ApiError)` |
| An attempt exceeded its timeout | `Error::Timeout { timeout }` |
| A network failure | `Error::Connection { source }` |
| Rejected before sending | `Error::Validation { field, message }` |
| A successful response that does not fit the typed response | `Error::Decode { source, body, request_id }`: the call was billed, do not retry it; the data is in `body` |

`ApiError` has `status`, `code` (the body's numeric `code`), `message`, `request_id`, `retry_after` and `body` (the raw bytes), and `is_authentication()` (401), `is_permission_denied()` (403), `is_bad_request()` (400, 422), `is_rate_limited()` (429) and `is_server_error()` (500 and above).

A 2xx response counts as a failure when its body is not a non-empty JSON object, or when it carries a numeric `code` other than 0. `message` is the body's `msg`, `message`, `detail` or `error` (the first non-empty string; when `error` is an object, the same keys inside it), else the first 500 UTF-16 code units of the body, else `HTTP <status>`. `request_id` comes from the `x-request-id` header, else the body. `retry_after` is set for 429 responses with a valid `Retry-After`, rounded up to whole seconds.

```rust,no_run
# async fn run(client: octen::Client) {
match client.search().search(&octen::SearchParams::new("hello")).await {
    Ok(resp) => println!("{}", resp.request_id),
    Err(octen::Error::Api(e)) if e.is_rate_limited() => println!("slow down: {:?}", e.retry_after),
    Err(octen::Error::Api(e)) => println!("{} {} ({:?})", e.status, e.message, e.request_id),
    Err(octen::Error::Timeout { timeout }) => println!("timed out after {timeout:?}"),
    Err(e) => println!("{e}"),
}
# }
```

## Retries and timeouts

**Retried automatically** (up to `max_retries` times, 3 by default): HTTP 408, 429, 500, 502, 503 and 504, and errors while establishing the connection, which never reached the server. Waits are `1 s * 2^(n-1)` before retry `n`, plus up to 25% jitter, at most 60 s; a `Retry-After` (seconds or an HTTP date) is honoured on 429 and 503, and if it asks for more than 60 s the call stops with the error. Every retry sends the same request body.

**Not retried:** an SDK timeout (the server may have finished and billed the request), a connection that failed after the request was sent, and any other status. Dropping the future cancels the call.

**Timeouts** cover each attempt, from connecting to reading the whole body. The call's `RequestOptions` timeout wins, then the client's `timeout`, then the endpoint default: 30 s; 120 s for Broad Search; for Extract, the per-URL `timeout` clamped to 1 to 60 s (30 s when unset) plus 90 s, so 120 s by default and 150 s with `timeout: Some(60)`. A client timeout shorter than that also applies to Extract and can stop it while the server is still working; set a per-call timeout in that case.

**Double billing.** The API has no idempotency keys yet. A gateway that answers 408, 502, 503 or 504 after the backend succeeded makes the retry bill again, and a connection that drops while the response is written leaves a billed call that looks failed. For billing-sensitive jobs use `max_retries(0)` and retry yourself.

## Differences from the other SDKs

Behaviour matches the Python (`octen-py` 0.13.1), TypeScript and Go SDKs, with these differences:

| Area | Python | TypeScript / Go | Rust |
|---|---|---|---|
| Local checks | Also value ranges (such as `count`) and enum values | Required inputs only | Required inputs only; enum fields are typed |
| Image and Video Search input | `query=` or `image_url=` keywords | Raw `inputs` | Raw `inputs`, built with `ImageSearchParams::text`, `image_url` or `image_data` |
| Embedding | Also sends `truncation` when passed, which the server ignores; helpers such as `embed_query` | The request body as in the API reference | The request body as in the API reference |
| Retried network errors | All, by default | Failures before the request is sent | Failures while establishing the connection |
| Proxies | Environment | Environment (Go), runtime defaults (TypeScript) | Environment variables only |
| Cancellation | — | `AbortSignal` / `context.Context` | Drop the future |
| Blocking client | Yes | — | No; asynchronous only |

## Versioning

The crate follows semantic versioning; before 1.0 a minor release may contain incompatible changes, listed in [CHANGELOG.md](CHANGELOG.md). The minimum supported Rust version is 1.85 and is checked in CI; raising it is a minor-version change.

## Development

```sh
cargo test                                   # unit, contract and mock-server tests
OCTEN_LIVE=1 OCTEN_LIVE_VIDEO=1 OCTEN_API_KEY=... \
  cargo test --test live -- --ignored --test-threads=1   # production API, billed
```

`testdata/openapi.json` is a snapshot of the public OpenAPI spec that `tests/contract.rs` checks the request fields, enum values and response examples against. A weekly workflow runs the same tests against the live spec (`OCTEN_SPEC_PATH`) and fails when it has drifted; it never rewrites the snapshot.

Releases are tagged `vX.Y.Z` on `main`; the release workflow checks the tag against `Cargo.toml` and publishes to crates.io.

## License

MIT
