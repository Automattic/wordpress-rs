#![allow(dead_code, unused_variables)]

use std::{borrow::Cow, collections::HashMap, str::FromStr};

pub use api_client::{WpApiClient, WpApiRequestBuilder};
pub use api_error::{ParsedRequestError, RequestExecutionError, WpApiError, WpError, WpErrorCode};
pub use parsed_url::{ParseUrlError, ParsedUrl};
use plugins::*;
use url_query::AsQueryValue;
use users::*;
pub use uuid::{WpUuid, WpUuidParseError};

mod api_client; // re-exported relevant types
mod api_error; // re-exported relevant types
mod parsed_url; // re-exported relevant types
mod uuid; // re-exported relevant types

pub mod application_passwords;
pub mod login;
pub mod plugins;
pub mod post_types;
pub mod posts;
pub mod request;
pub mod site_settings;
pub mod url_query;
pub mod users;
pub mod wp_site_health_tests;

#[cfg(test)]
mod unit_test_common;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WpContext {
    Edit,
    Embed,
    #[default]
    View,
}

impl WpContext {
    fn as_str(&self) -> &str {
        match self {
            Self::Edit => "edit",
            Self::Embed => "embed",
            Self::View => "view",
        }
    }
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum WpAuthentication {
    AuthorizationHeader { token: String },
    None,
}

impl WpAuthentication {
    pub fn from_username_and_password(username: String, password: String) -> Self {
        use base64::prelude::*;
        WpAuthentication::AuthorizationHeader {
            token: BASE64_STANDARD.encode(format!("{}:{}", username, password)),
        }
    }
}

#[uniffi::export]
fn wp_authentication_from_username_and_password(
    username: String,
    password: String,
) -> WpAuthentication {
    WpAuthentication::from_username_and_password(username, password)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WpApiParamOrder {
    #[default]
    Asc,
    Desc,
}

impl_as_query_value_from_as_str!(WpApiParamOrder);

impl WpApiParamOrder {
    fn as_str(&self) -> &str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

// TODO: Improve error handling
impl FromStr for WpApiParamOrder {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "asc" => Ok(Self::Asc),
            "desc" => Ok(Self::Desc),
            _ => Err(()),
        }
    }
}

trait SparseField {
    fn as_str(&self) -> &str;
}

#[derive(Debug)]
pub struct UrlQueryPairsMap<'a> {
    inner: HashMap<Cow<'a, str>, Cow<'a, str>>,
}

impl<'a> UrlQueryPairsMap<'a> {
    fn new(query_pairs: HashMap<Cow<'a, str>, Cow<'a, str>>) -> Self {
        Self { inner: query_pairs }
    }

    fn get<T: FromStr>(&self, key: &str) -> Option<T> {
        self.inner.get(key).and_then(|v| v.parse().ok())
    }

    fn get_csv<T: FromStr>(&self, key: &str) -> Vec<T> {
        self.inner
            .get(key)
            .and_then(|v| {
                v.split(',')
                    .map(|s| T::from_str(s).ok())
                    .collect::<Option<Vec<_>>>()
            })
            .unwrap_or_default()
    }
}

pub trait FromUrlQueryPairs
where
    Self: Sized,
{
    fn from_url_query_pairs(query_pairs: UrlQueryPairsMap) -> Option<Self>;
}

impl FromUrlQueryPairs for () {
    fn from_url_query_pairs(query_pairs: UrlQueryPairsMap) -> Option<Self> {
        None
    }
}

#[macro_export]
macro_rules! generate {
    ($type_name:ident) => {
        $type_name::default()
    };
    ($type_name:ident, $(($f:ident, $v:expr)), *) => {{
        let mut obj = $type_name::default();
        $(obj.$f = $v;)*
        obj
    }};
}

uniffi::setup_scaffolding!();
