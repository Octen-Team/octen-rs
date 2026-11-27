//! Business Search.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::client::Params;
use crate::error::Error;
use crate::types::{
    require_query, response_struct, FullContentOptions, HighlightOptions, Language, Response,
    TimeBasis, TimeRange, WebResult,
};

/// Entity card options of a Business Search.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct BusinessEntityOptions {
    /// Whether to return entity cards.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    /// The number of entities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// The most activities per entity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_activities: Option<i64>,
    /// The most news items per entity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_news: Option<i64>,
}

/// The request of Business Search.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct BusinessSearchParams {
    /// The search query; required.
    pub query: String,
    /// The number of results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// Only results from these domains.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub include_domains: Vec<String>,
    /// No results from these domains.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclude_domains: Vec<String>,
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
    /// Result languages.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub language: Vec<Language>,
    /// Highlight options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<HighlightOptions>,
    /// Full-content options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_content: Option<FullContentOptions>,
    /// Entity card options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entities: Option<BusinessEntityOptions>,
}

impl BusinessSearchParams {
    /// A request for `query` with every other field unset.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            ..Self::default()
        }
    }
}

impl Params for BusinessSearchParams {
    fn validate(&self) -> Result<(), Error> {
        require_query(&self.query)
    }
}

response_struct! {
    /// The external identifiers of a company.
    BusinessEntityIdentifiers {
        /// The website.
        website: String,
        /// The LinkedIn page.
        linkedin_url: String,
        /// The stock ticker.
        stock_ticker: String,
        /// The SEC CIK.
        sec_cik: String,
    }
}

response_struct! {
    /// The descriptive attributes of a company.
    BusinessEntityAttributes {
        /// The industry.
        industry: String,
        /// The headquarters location.
        hq_location: String,
        /// The founding year.
        founded_year: i64,
        /// The employee count range.
        employee_range: String,
    }
}

response_struct! {
    /// One reporting period of company financials.
    BusinessFinancials {
        /// Revenue.
        revenue: f64,
        /// Net income.
        net_income: f64,
        /// Gross profit.
        gross_profit: f64,
        /// The reporting period.
        period: String,
        /// The currency.
        currency: String,
    }
}

response_struct! {
    /// A stock quote.
    BusinessStock {
        /// The price.
        price: f64,
        /// Today's change in percent.
        todays_change_percent: f64,
        /// The 52-week high.
        week_52_high: f64,
        /// The 52-week low.
        week_52_low: f64,
        /// The quote date.
        date: String,
        /// The currency.
        currency: String,
    }
}

response_struct! {
    /// One funding round.
    BusinessFunding {
        /// The round.
        round: String,
        /// The amount raised.
        amount: f64,
        /// The date.
        date: String,
        /// The lead investors.
        lead_investors: String,
        /// The currency.
        currency: String,
    }
}

response_struct! {
    /// A web traffic estimate.
    BusinessWebTraffic {
        /// Monthly visits.
        visits_monthly: i64,
        /// The traffic rank.
        rank: i64,
        /// The period.
        period: String,
    }
}

response_struct! {
    /// The numeric data of a company.
    BusinessEntityMetrics {
        /// Financials by period.
        financials: Vec<BusinessFinancials>,
        /// The stock quote.
        stock: BusinessStock,
        /// Funding rounds.
        funding: Vec<BusinessFunding>,
        /// Web traffic.
        web_traffic: BusinessWebTraffic,
    }
}

response_struct! {
    /// A key person of a company.
    BusinessKeyPerson {
        /// The name.
        name: String,
        /// The title.
        title: String,
    }
}

response_struct! {
    /// One position in a person's career.
    BusinessCareerEntry {
        /// The organization.
        organization: String,
        /// The title.
        title: String,
        /// The start.
        start: String,
        /// The end.
        end: String,
    }
}

response_struct! {
    /// An image URL, such as a company logo or a person's photo.
    BusinessImageLink {
        /// The URL.
        url: String,
    }
}

response_struct! {
    /// A company card.
    CompanyEntity {
        /// The name.
        name: String,
        /// A summary.
        summary: String,
        /// Other names.
        aliases: Vec<String>,
        /// External identifiers.
        identifiers: BusinessEntityIdentifiers,
        /// Descriptive attributes.
        attributes: BusinessEntityAttributes,
        /// Numeric data.
        metrics: BusinessEntityMetrics,
        /// Key people.
        key_people: Vec<BusinessKeyPerson>,
        /// The logo.
        logo: BusinessImageLink,
        /// Recent activities.
        activities: Vec<WebResult>,
        /// News items.
        news: Vec<WebResult>,
    }
}

response_struct! {
    /// A person card.
    PersonEntity {
        /// The name.
        name: String,
        /// A summary.
        summary: String,
        /// The LinkedIn page.
        linkedin_url: String,
        /// The current position.
        current_position: String,
        /// The career.
        career: Vec<BusinessCareerEntry>,
        /// The photo.
        photo: BusinessImageLink,
        /// Recent activities.
        activities: Vec<WebResult>,
        /// News items.
        news: Vec<WebResult>,
    }
}

/// An entity card, selected by its `type`. A type this version does not know, or a missing one,
/// decodes as [`BusinessEntity::Other`] with the entity's JSON, so new types never break decoding:
/// always keep a fallback arm when matching.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum BusinessEntity {
    /// `type: "company"`.
    Company(Box<CompanyEntity>),
    /// `type: "person"`.
    Person(Box<PersonEntity>),
    /// Any other or missing `type`.
    Other {
        /// The `type` string, when there is one.
        entity_type: Option<String>,
        /// The entity's JSON as sent.
        raw: serde_json::Value,
    },
}

impl<'de> Deserialize<'de> for BusinessEntity {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = serde_json::Value::deserialize(deserializer)?;
        let entity_type = raw.get("type").and_then(|t| t.as_str()).map(str::to_owned);
        match entity_type.as_deref() {
            Some("company") => serde_json::from_value(raw)
                .map(|c| Self::Company(Box::new(c)))
                .map_err(D::Error::custom),
            Some("person") => serde_json::from_value(raw)
                .map(|p| Self::Person(Box::new(p)))
                .map_err(D::Error::custom),
            _ => Ok(Self::Other { entity_type, raw }),
        }
    }
}

impl Serialize for BusinessEntity {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        #[serde(tag = "type", rename_all = "lowercase")]
        enum Tagged<'a> {
            Company(&'a CompanyEntity),
            Person(&'a PersonEntity),
        }
        match self {
            Self::Company(c) => Tagged::Company(c).serialize(serializer),
            Self::Person(p) => Tagged::Person(p).serialize(serializer),
            Self::Other { raw, .. } => raw.serialize(serializer),
        }
    }
}

response_struct! {
    /// The data of a Business Search response.
    BusinessSearchData {
        /// The query that was run.
        query: String,
        /// The web results.
        results: Vec<WebResult>,
        /// The entity cards.
        entities: Vec<BusinessEntity>,
    }
}

response_struct! {
    /// What a Business Search call is billed for.
    BusinessSearchUsage {
        /// The number of searches run.
        num_search_queries: i64,
        /// The results whose full content was extracted beyond the included amount.
        full_content_extra_count: i64,
    }
}

response_struct! {
    /// The meta of a Business Search response.
    BusinessSearchMeta {
        /// What the call is billed for.
        usage: BusinessSearchUsage,
        /// Server latency in milliseconds.
        latency: f64,
        /// A warning about the request.
        warning: String,
    }
}

/// The response of Business Search.
pub type BusinessSearchResponse = Response<BusinessSearchData, BusinessSearchMeta>;
