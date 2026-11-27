//! `OCTEN_API_KEY=... cargo run --example embedding`

use octen::{Client, EmbeddingModel, EmbeddingParams, InputType};

#[tokio::main]
async fn main() -> Result<(), octen::Error> {
    let client = Client::from_env()?;
    let params = EmbeddingParams {
        model: Some(EmbeddingModel::OctenEmbedding0_6B),
        input_type: Some(InputType::Document),
        ..EmbeddingParams::new([
            "Octen is a search API.",
            "Vector search finds items by meaning.",
        ])
    };
    let resp = client.embedding().create(&params).await?;
    for r in resp.data.and_then(|d| d.results).unwrap_or_default() {
        let v = r.embedding.unwrap_or_default();
        println!(
            "input {:?}: {} dimensions, first {:?}",
            r.index,
            v.len(),
            v.first()
        );
    }
    println!("request_id: {}", resp.request_id);
    Ok(())
}
