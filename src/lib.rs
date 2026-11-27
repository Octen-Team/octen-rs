//! Official Rust SDK for the [Octen API](https://docs.octen.ai).

mod client;
mod config;
mod envelope;
mod error;
mod retry;
mod transport;

pub use client::Client;
pub use config::{ClientBuilder, RequestOptions};
pub use error::{ApiError, Error};
