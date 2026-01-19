use crate::prelude::*;
use crate::{impl_as_query_value_for_new_type, request::endpoint::AsNamespace};
use serde::{Deserialize, Serialize};
use std::{num::ParseIntError, str::FromStr, sync::Arc};

pub mod client;
pub mod endpoint;
pub mod followers;
pub mod jetpack_connection;
pub mod me;
pub mod oauth2;
pub mod sites;
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

pub(crate) enum WpComNamespace {
    Oauth2,
    RestV1_1,
    RestV1_2,
    V2,
}

impl AsNamespace for WpComNamespace {
    fn namespace_value(&self) -> &'static str {
        match self {
            WpComNamespace::Oauth2 => "/oauth2",
            WpComNamespace::RestV1_1 => "/rest/v1.1",
            WpComNamespace::RestV1_2 => "/rest/v1.2",
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
