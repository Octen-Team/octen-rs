//! Its own test binary: it changes process-wide proxy variables, which other tests must not see.

use octen::{Client, SearchParams};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

// Like Go's default client: HTTP(S)_PROXY from the environment is honoured; operating-system proxy settings are not read.
#[tokio::test]
async fn honours_proxy_environment_variables() {
    let proxy = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"code":0,"request_id":"r","data":{}}"#,
            "application/json",
        ))
        .mount(&proxy)
        .await;
    std::env::set_var("HTTP_PROXY", proxy.uri());
    std::env::remove_var("NO_PROXY");
    let client = Client::builder()
        .api_key("k")
        .base_url("http://api.octen.invalid")
        .build();
    std::env::remove_var("HTTP_PROXY");
    client
        .unwrap()
        .search()
        .search(&SearchParams::new("q"))
        .await
        .unwrap();
    let reqs = proxy.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].headers.get("host").unwrap(), "api.octen.invalid");
}
