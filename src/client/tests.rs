use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use super::*;
use crate::transport::{Hooks, USER_AGENT_VALUE};

const OK: &str = r#"{"code":0,"msg":"success","request_id":"rid-1","data":{"n":1}}"#;

#[derive(Serialize)]
struct Query {
    query: String,
}

impl Params for Query {
    fn validate(&self) -> Result<(), Error> {
        if self.query.trim().is_empty() {
            return Err(Error::validation("query", "query is required."));
        }
        Ok(())
    }
}

#[derive(Deserialize, Debug)]
struct Reply {
    request_id: String,
}

fn q() -> Query {
    Query { query: "q".into() }
}

struct Harness {
    client: Client,
    waits: Arc<Mutex<Vec<Duration>>>,
}

fn harness(base_url: &str, configure: impl FnOnce(ClientBuilder) -> ClientBuilder) -> Harness {
    let waits = Arc::new(Mutex::new(Vec::new()));
    let mut builder = configure(Client::builder().api_key("test-key").base_url(base_url));
    builder.hooks = Hooks {
        waits: Some(waits.clone()),
        jitter: Some(0.0),
        timeouts: None,
    };
    // Building reads proxy settings from the environment, which another test may be changing
    let client = {
        let _guard = env_lock();
        builder.build().unwrap()
    };
    Harness { client, waits }
}

async fn call(c: &Client, opts: Option<&RequestOptions>) -> Result<Reply, Error> {
    c.post::<_, Reply>("/search", &q(), opts, Duration::from_secs(30))
        .await
        .map(|(r, _)| r)
}

fn json(status: u16, body: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_raw(body.to_owned(), "application/json")
}

/// Responds with the given responses in order, then repeats the last one.
fn sequence(responses: Vec<ResponseTemplate>) -> impl Fn(&Request) -> ResponseTemplate {
    let n = Arc::new(Mutex::new(0usize));
    move |_: &Request| {
        let mut i = n.lock().unwrap();
        let r = responses[(*i).min(responses.len() - 1)].clone();
        *i += 1;
        r
    }
}

// Environment variables are process-wide: tests that change them, and every client build, take this lock.
static ENV: Mutex<()> = Mutex::new(());

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn from_env_reads_octen_api_key() {
    let _guard = env_lock();
    std::env::set_var("OCTEN_API_KEY", "  env-key  ");
    let from_env = Client::from_env();
    // A blank explicit key falls back to the environment too
    let blank = Client::new("   ");
    std::env::remove_var("OCTEN_API_KEY");
    assert!(from_env.is_ok());
    assert!(blank.is_ok());
}

#[test]
fn missing_key_is_validation_error() {
    let _guard = env_lock();
    std::env::remove_var("OCTEN_API_KEY");
    for result in [Client::from_env(), Client::new(""), Client::new(" \t")] {
        match result {
            Err(Error::Validation { field, .. }) => assert_eq!(field, "api_key"),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }
    match Client::new("bad\nkey") {
        Err(Error::Validation { field, .. }) => assert_eq!(field, "api_key"),
        other => panic!("expected a validation error, got {other:?}"),
    }
}

#[test]
fn refuses_x_api_key_header() {
    for name in ["x-api-key", "X-API-KEY"] {
        match Client::builder().api_key("k").header(name, "v").build() {
            Err(Error::Validation { field, .. }) => assert_eq!(field, "header"),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }
}

#[test]
fn rejects_invalid_configuration() {
    let cases: Vec<(ClientBuilder, &str)> = vec![
        (Client::builder().base_url("ftp://example.com"), "base_url"),
        (Client::builder().base_url("api.octen.ai"), "base_url"),
        (
            Client::builder().base_url("https://user:pw@example.com"),
            "base_url",
        ),
        (
            Client::builder().base_url("https://example.com/?a=1"),
            "base_url",
        ),
        (
            Client::builder().base_url("https://example.com/#x"),
            "base_url",
        ),
        (
            Client::builder().base_url("https://exa mple.com"),
            "base_url",
        ),
        (Client::builder().timeout(Duration::ZERO), "timeout"),
        (Client::builder().header("bad name", "v"), "header"),
        (Client::builder().header("x-trace", "a\nb"), "header"),
    ];
    for (builder, field) in cases {
        match builder.api_key("k").build() {
            Err(Error::Validation { field: got, .. }) => assert_eq!(got, field),
            other => panic!("expected a validation error on {field}, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn sends_x_api_key_user_agent_and_json() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(json(200, OK))
        .mount(&server)
        .await;
    // A trailing slash on the base URL is dropped
    let h = harness(&format!("{}/", server.uri()), |b| {
        b.api_key("  test-key  ").header("x-trace", "client")
    });
    let opts = RequestOptions::new()
        .header("x-trace", "call")
        .header("x-extra", "1");
    let reply = call(&h.client, Some(&opts)).await.unwrap();
    assert_eq!(reply.request_id, "rid-1");

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1);
    let r = &reqs[0];
    let header = |name: &str| r.headers.get(name).map(|v| v.to_str().unwrap().to_owned());
    assert_eq!(r.url.path(), "/search");
    assert_eq!(header("x-api-key").as_deref(), Some("test-key"));
    assert_eq!(header("user-agent").as_deref(), Some(USER_AGENT_VALUE));
    assert_eq!(
        USER_AGENT_VALUE,
        concat!("octen-rust-sdk/", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(header("content-type").as_deref(), Some("application/json"));
    assert_eq!(header("accept").as_deref(), Some("application/json"));
    // A per-call header replaces the client header of the same name
    assert_eq!(header("x-trace").as_deref(), Some("call"));
    assert_eq!(header("x-extra").as_deref(), Some("1"));
    assert_eq!(r.body, br#"{"query":"q"}"#);
}

#[tokio::test]
async fn per_call_options_are_validated() {
    let server = MockServer::start().await;
    let h = harness(&server.uri(), |b| b);
    for (opts, field) in [
        (RequestOptions::new().timeout(Duration::ZERO), "timeout"),
        (
            RequestOptions::new().header("x-api-key", "other"),
            "headers",
        ),
        (RequestOptions::new().header("bad name", "v"), "headers"),
    ] {
        match call(&h.client, Some(&opts)).await {
            Err(Error::Validation { field: got, .. }) => assert_eq!(got, field),
            other => panic!("expected a validation error on {field}, got {other:?}"),
        }
    }
    // Invalid params are rejected before sending too
    let blank = Query { query: " ".into() };
    let r = h
        .client
        .post::<_, Reply>("/search", &blank, None, Duration::from_secs(30))
        .await;
    assert!(
        matches!(r, Err(Error::Validation { ref field, .. }) if field == "query"),
        "{r:?}"
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn per_call_timeout_overrides_default() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(json(200, OK).set_delay(Duration::from_millis(400)))
        .mount(&server)
        .await;
    // The endpoint default applies when nothing else is set
    let h = harness(&server.uri(), |b| b);
    let r = h
        .client
        .post::<_, Reply>("/search", &q(), None, Duration::from_millis(100))
        .await;
    assert!(
        matches!(r, Err(Error::Timeout { timeout }) if timeout == Duration::from_millis(100)),
        "{r:?}"
    );
    // A client timeout overrides the endpoint default
    let h = harness(&server.uri(), |b| b.timeout(Duration::from_millis(150)));
    let r = h
        .client
        .post::<_, Reply>("/search", &q(), None, Duration::from_secs(30))
        .await;
    assert!(
        matches!(r, Err(Error::Timeout { timeout }) if timeout == Duration::from_millis(150)),
        "{r:?}"
    );
    // A per-call timeout overrides both
    let opts = RequestOptions::new().timeout(Duration::from_secs(5));
    assert!(call(&h.client, Some(&opts)).await.is_ok());
}

#[tokio::test]
async fn retries_retryable_statuses_up_to_max() {
    for status in [408, 429, 500, 502, 503, 504] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(sequence(vec![
                json(status, r#"{"msg":"busy"}"#),
                json(status, r#"{"msg":"busy"}"#),
                json(200, OK),
            ]))
            .mount(&server)
            .await;
        let h = harness(&server.uri(), |b| b);
        assert!(call(&h.client, None).await.is_ok(), "{status}");
        assert_eq!(
            server.received_requests().await.unwrap().len(),
            3,
            "{status}"
        );
        // Backoff 1 s, then 2 s (jitter fixed at 0)
        assert_eq!(
            *h.waits.lock().unwrap(),
            vec![Duration::from_secs(1), Duration::from_secs(2)],
            "{status}"
        );
    }
    // max_retries bounds the attempts: 1 + 2 retries, then the last error
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(json(503, r#"{"msg":"down"}"#))
        .mount(&server)
        .await;
    let h = harness(&server.uri(), |b| b.max_retries(2));
    let e = call(&h.client, None).await.unwrap_err();
    assert!(
        e.as_api()
            .is_some_and(|a| a.status == 503 && a.message == "down"),
        "{e:?}"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
    // max_retries 0 sends once
    let h = harness(&server.uri(), |b| b.max_retries(0));
    assert!(call(&h.client, None).await.is_err());
    assert_eq!(server.received_requests().await.unwrap().len(), 4);
}

#[tokio::test]
async fn does_not_retry_400() {
    for status in [400, 401, 403, 404, 422] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(json(status, r#"{"code":1002,"msg":"bad"}"#))
            .mount(&server)
            .await;
        let h = harness(&server.uri(), |b| b);
        let e = call(&h.client, None).await.unwrap_err();
        let api = e.as_api().expect("an API error");
        assert_eq!(
            (api.status, api.code, api.message.as_str()),
            (status, Some(1002.0), "bad")
        );
        assert_eq!(
            server.received_requests().await.unwrap().len(),
            1,
            "{status}"
        );
        assert!(h.waits.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn does_not_retry_client_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(json(200, OK).set_delay(Duration::from_millis(500)))
        .mount(&server)
        .await;
    let h = harness(&server.uri(), |b| b.timeout(Duration::from_millis(100)));
    let r = call(&h.client, None).await;
    assert!(matches!(r, Err(Error::Timeout { .. })), "{r:?}");
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert!(h.waits.lock().unwrap().is_empty());
}

#[tokio::test]
async fn retries_connect_error() {
    // A port nobody listens on: bind, note the port, close
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let h = harness(&format!("http://127.0.0.1:{port}"), |b| b.max_retries(2));
    let r = call(&h.client, None).await;
    assert!(matches!(r, Err(Error::Connection { .. })), "{r:?}");
    assert_eq!(
        h.waits.lock().unwrap().len(),
        2,
        "two retries after the first failure"
    );
}

#[tokio::test]
async fn honours_retry_after_on_429_and_503() {
    for status in [429, 503] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(sequence(vec![
                json(status, r#"{"msg":"slow down"}"#).insert_header("retry-after", "7"),
                json(200, OK),
            ]))
            .mount(&server)
            .await;
        let h = harness(&server.uri(), |b| b);
        assert!(call(&h.client, None).await.is_ok());
        assert_eq!(
            *h.waits.lock().unwrap(),
            vec![Duration::from_secs(7)],
            "{status}"
        );
    }
    // 500 ignores Retry-After and uses the backoff
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(sequence(vec![
            json(500, "{}").insert_header("retry-after", "7"),
            json(200, OK),
        ]))
        .mount(&server)
        .await;
    let h = harness(&server.uri(), |b| b);
    assert!(call(&h.client, None).await.is_ok());
    assert_eq!(*h.waits.lock().unwrap(), vec![Duration::from_secs(1)]);
}

#[tokio::test]
async fn gives_up_when_retry_after_exceeds_60s() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(json(429, r#"{"msg":"slow down"}"#).insert_header("retry-after", "61"))
        .mount(&server)
        .await;
    let h = harness(&server.uri(), |b| b);
    let e = call(&h.client, None).await.unwrap_err();
    let api = e.as_api().expect("an API error");
    assert!(api.is_rate_limited());
    assert_eq!(api.retry_after, Some(Duration::from_secs(61)));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert!(h.waits.lock().unwrap().is_empty());
}

#[tokio::test]
async fn retry_resends_identical_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(sequence(vec![
            json(502, "{}"),
            json(500, "{}"),
            json(200, OK),
        ]))
        .mount(&server)
        .await;
    let h = harness(&server.uri(), |b| b);
    // A large body, like base64 image data or a thousand texts
    let big = Query {
        query: "x".repeat(2 * 1024 * 1024),
    };
    assert!(h
        .client
        .post::<_, Reply>("/search", &big, None, Duration::from_secs(30))
        .await
        .is_ok());
    let reqs = server.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 3);
    let want = serde_json::to_vec(&big).unwrap();
    for r in &reqs {
        assert_eq!(r.body, want);
        assert_eq!(r.headers.get("x-api-key").unwrap(), "test-key");
    }
}

#[tokio::test]
async fn http_200_with_nonzero_code_is_api_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(json(
            200,
            r#"{"code":1001,"msg":"invalid query","request_id":"abc"}"#,
        ))
        .mount(&server)
        .await;
    let h = harness(&server.uri(), |b| b);
    let e = call(&h.client, None).await.unwrap_err();
    let api = e.as_api().expect("an API error");
    assert_eq!(
        (api.status, api.code, api.message.as_str()),
        (200, Some(1001.0), "invalid query")
    );
    assert_eq!(api.request_id.as_deref(), Some("abc"));
    assert!(!api.is_bad_request() && !api.is_server_error());
    // Not retried
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn undecodable_2xx_is_decode_error_with_body() {
    let server = MockServer::start().await;
    let body = r#"{"code":0,"request_id":7,"data":{}}"#;
    Mock::given(method("POST"))
        .respond_with(json(200, body).insert_header("x-request-id", "hdr-id"))
        .mount(&server)
        .await;
    let h = harness(&server.uri(), |b| b);
    match call(&h.client, None).await {
        Err(Error::Decode {
            body: got,
            request_id,
            ..
        }) => {
            assert_eq!(got, body.as_bytes());
            assert_eq!(request_id.as_deref(), Some("hdr-id"));
        }
        other => panic!("expected a decode error, got {other:?}"),
    }
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn redirects_are_not_followed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(301)
                .insert_header("location", "https://elsewhere.example/search"),
        )
        .mount(&server)
        .await;
    let h = harness(&server.uri(), |b| b);
    let e = call(&h.client, None).await.unwrap_err();
    let api = e.as_api().expect("an API error");
    assert_eq!(api.status, 301);
    assert_eq!(
        api.message,
        "HTTP 301 redirect to https://elsewhere.example/search; redirects are not followed, check base_url"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

// The request reached the server and the connection dropped before a response: retrying could bill twice.
#[tokio::test]
async fn does_not_retry_connection_dropped_after_send() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::AsyncReadExt;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    let counter = accepted.clone();
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = listener.accept().await.unwrap();
            counter.fetch_add(1, Ordering::SeqCst);
            let mut buf = vec![0u8; 64 * 1024];
            let _ = sock.read(&mut buf).await;
            drop(sock);
        }
    });
    let h = harness(&format!("http://{addr}"), |b| b);
    let r = call(&h.client, None).await;
    assert!(matches!(r, Err(Error::Connection { .. })), "{r:?}");
    assert_eq!(accepted.load(Ordering::SeqCst), 1);
    assert!(h.waits.lock().unwrap().is_empty());
}

#[test]
fn debug_output_hides_the_api_key() {
    let builder = Client::builder().api_key("sk-secret-123");
    let shown = format!("{builder:?}");
    assert!(!shown.contains("sk-secret-123"), "{shown}");
    assert!(shown.contains("api_key"), "{shown}");
    let client = builder.build().unwrap();
    assert!(!format!("{client:?}").contains("sk-secret-123"));
}

// A client from http_client_builder keeps the SDK's no-redirect policy, so the key never reaches
// another origin; reqwest strips only Authorization and cookies on a cross-origin redirect.
#[tokio::test]
async fn custom_http_client_does_not_follow_redirects() {
    let other = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(json(200, OK))
        .mount(&other)
        .await;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(307).insert_header("location", format!("{}/search", other.uri())),
        )
        .mount(&server)
        .await;
    let h = harness(&server.uri(), |b| {
        b.http_client_builder(reqwest::Client::builder())
    });
    let e = call(&h.client, None).await.unwrap_err();
    assert_eq!(e.as_api().map(|a| a.status), Some(307), "{e:?}");
    assert!(
        other.received_requests().await.unwrap().is_empty(),
        "the key reached the redirect target"
    );
}
