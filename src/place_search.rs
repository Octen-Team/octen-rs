//! Place Search.

use serde::Serialize;

use crate::client::Params;
use crate::error::Error;
use crate::types::{
    require_finite, require_query, response_struct, string_enum, FullContentOptions,
    HighlightOptions, Language, Response, TimeBasis, TimeRange, WebResult,
};

/// Place card options.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct PlaceCardOptions {
    /// The number of places.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// The most reviews per place.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_reviews: Option<i64>,
}

/// Options for the web search that accompanies a Place Search.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct PlaceWebSearchOptions {
    /// Whether to run the web search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    /// The number of web results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// Highlight options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<HighlightOptions>,
    /// Full-content options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_content: Option<FullContentOptions>,
    /// Only results from these domains.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub include_domains: Vec<String>,
    /// No results from these domains.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclude_domains: Vec<String>,
    /// Result languages.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub language: Vec<Language>,
    /// Which date the time filters apply to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_basis: Option<TimeBasis>,
    /// A recent time window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<TimeRange>,
    /// The start of a time window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    /// The end of a time window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
}

string_enum! {
    /// The kind of user location.
    UserLocationType { Approximate = "approximate" }
}

/// An approximate user location.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct PlaceUserLocation {
    /// The kind of location.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub location_type: Option<UserLocationType>,
    /// The city.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    /// The region.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// The country.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// The IANA time zone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

/// The request of Place Search.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct PlaceSearchParams {
    /// The search query; required.
    pub query: String,
    /// Place card options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub places: Option<PlaceCardOptions>,
    /// Options for the accompanying web search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search: Option<PlaceWebSearchOptions>,
    /// An approximate user location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_location: Option<PlaceUserLocation>,
    /// The latitude to search around.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    /// The longitude to search around.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    /// The search radius in meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<i64>,
}

impl PlaceSearchParams {
    /// A request for `query` with every other field unset.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            ..Self::default()
        }
    }
}

impl Params for PlaceSearchParams {
    // JSON has no NaN or infinity: serde_json would send null in their place.
    fn validate(&self) -> Result<(), Error> {
        require_query(&self.query)?;
        require_finite("latitude", self.latitude)?;
        require_finite("longitude", self.longitude)
    }
}

response_struct! {
    /// The location a Place Search was resolved to.
    PlaceSearchLocation {
        /// The latitude.
        latitude: f64,
        /// The longitude.
        longitude: f64,
    }
}

response_struct! {
    /// One place card. `attributes` holds amenity flags decided by the server.
    Place {
        /// The name.
        name: String,
        /// The category.
        category: String,
        /// The address.
        address: String,
        /// The latitude.
        latitude: f64,
        /// The longitude.
        longitude: f64,
        /// The distance in meters.
        distance: i64,
        /// The phone number.
        phone: String,
        /// The website.
        website: String,
        /// Opening hours.
        opening_hours: Vec<String>,
        /// The business status.
        status: String,
        /// The rating.
        rating: f64,
        /// The number of reviews.
        review_count: i64,
        /// Amenity flags.
        attributes: serde_json::Map<String, serde_json::Value>,
    }
}

response_struct! {
    /// The data of a Place Search response.
    PlaceSearchData {
        /// The query that was run.
        query: String,
        /// The resolved location.
        location: PlaceSearchLocation,
        /// The place cards.
        places: Vec<Place>,
        /// The web results.
        results: Vec<WebResult>,
    }
}

response_struct! {
    /// What a Place Search call is billed for.
    PlaceSearchUsage {
        /// The number of searches run.
        num_search_queries: i64,
        /// The results whose full content was extracted beyond the included amount.
        full_content_extra_count: i64,
    }
}

response_struct! {
    /// The meta of a Place Search response.
    PlaceSearchMeta {
        /// What the call is billed for.
        usage: PlaceSearchUsage,
        /// Server latency in milliseconds.
        latency: f64,
        /// A warning about the request.
        warning: String,
    }
}

/// The response of Place Search.
pub type PlaceSearchResponse = Response<PlaceSearchData, PlaceSearchMeta>;
