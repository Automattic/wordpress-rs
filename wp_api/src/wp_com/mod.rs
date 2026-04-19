use crate::prelude::*;
use crate::{impl_as_query_value_for_new_type, request::endpoint::AsNamespace};
use serde::{Deserialize, Serialize};
use std::{num::ParseIntError, str::FromStr, sync::Arc};

pub mod client;
pub mod domains;
pub mod endpoint;
pub mod followers;
pub mod jetpack_connection;
pub mod language;
pub mod me;
pub mod me_connections;
pub mod oauth2;
pub mod products;
pub mod publicize;
pub mod segments;
pub mod sites;
pub mod stats_city_views;
pub mod stats_clicks;
pub mod stats_country_views;
pub mod stats_devices;
pub mod stats_emails_summary;
pub mod stats_file_downloads;
pub mod stats_insights;
pub mod stats_referrers;
pub mod stats_region_views;
pub mod stats_search_terms;
pub mod stats_subscribers;
pub mod stats_summary;
pub mod stats_tags;
pub mod stats_top_authors;
pub mod stats_top_posts;
pub mod stats_utm;
pub mod stats_video_plays;
pub mod stats_visits;
pub mod subscribers;
pub mod support_bots;
pub mod support_eligibility;
pub mod support_tickets;

impl_as_query_value_for_new_type!(WpComSiteId);
uniffi::custom_newtype!(WpComSiteId, u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WpComSiteId(pub u64);

impl FromStr for WpComSiteId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl std::fmt::Display for WpComSiteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

uniffi::custom_newtype!(CurrencyCode, String);
/// ISO 4217 currency code (e.g. `"USD"`, `"TRY"`, `"EUR"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CurrencyCode(pub String);

impl std::fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unit of a time span in the billing system.
///
/// Corresponds to the backend's `Time_Span_Unit` enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum TimeSpanUnit {
    Day,
    Week,
    Month,
    Year,
    Indefinite,
    /// A unit not covered by the known variants.
    #[serde(untagged)]
    Other(String),
}

pub(crate) enum WpComNamespace {
    Oauth2,
    RestV1_1,
    RestV1_2,
    RestV1_3,
    V2,
}

impl AsNamespace for WpComNamespace {
    fn namespace_value(&self) -> &'static str {
        match self {
            WpComNamespace::Oauth2 => "/oauth2",
            WpComNamespace::RestV1_1 => "/rest/v1.1",
            WpComNamespace::RestV1_2 => "/rest/v1.2",
            WpComNamespace::RestV1_3 => "/rest/v1.3",
            WpComNamespace::V2 => "/wpcom/v2",
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, uniffi::Enum)]
pub enum WpComBaseUrl {
    #[default]
    Production,
    Custom(Arc<ParsedUrl>),
}

impl WpComBaseUrl {
    pub fn parsed_url(&self) -> ParsedUrl {
        match self {
            WpComBaseUrl::Production => url::Url::parse("https://public-api.wordpress.com")
                .expect("This is a valid URL")
                .into(),
            WpComBaseUrl::Custom(url) => (**url).clone(),
        }
    }
}
