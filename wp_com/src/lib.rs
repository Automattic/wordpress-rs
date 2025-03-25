use serde::{Deserialize, Serialize};
use std::{num::ParseIntError, str::FromStr};
use wp_api::{impl_as_query_value_for_new_type, request::endpoint::AsNamespace};

pub mod client;
pub mod endpoint;
pub mod jetpack_connection;

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
    V2,
}

impl AsNamespace for WpComNamespace {
    fn as_str(&self) -> &str {
        match self {
            WpComNamespace::V2 => "/wpcom/v2",
        }
    }
}

uniffi::setup_scaffolding!();
