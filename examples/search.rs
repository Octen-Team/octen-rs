//! `OCTEN_API_KEY=... cargo run --example search -- "your query"`

use octen::{Client, SearchParams};

#[tokio::main]
async fn main() -> Result<(), octen::Error> {
    let query = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "rust async runtimes".into());
    let client = Client::from_env()?;
    let resp = client
        .search()
        .search(&SearchParams {
            count: Some(5),
            ..SearchParams::new(query)
        })
        .await?;
    for r in resp.data.and_then(|d| d.results).unwrap_or_default() {
        println!(
            "{}\n  {}",
            r.title.unwrap_or_default(),
            r.url.unwrap_or_default()
        );
    }
    println!("request_id: {}", resp.request_id);
    Ok(())
}
