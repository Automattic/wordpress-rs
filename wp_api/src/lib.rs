#![allow(dead_code, unused_variables)]

pub use api_client::{WpApiClient, WpApiRequestBuilder};
pub use api_error::{
    MediaUploadRequestExecutionError, ParsedRequestError, RequestExecutionError, WpApiError,
    WpError, WpErrorCode,
};
pub use parsed_url::{ParseUrlError, ParsedUrl};
use plugins::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};
use url_query::AsQueryValue;
use users::*;
pub use uuid::{WpUuid, WpUuidParseError};

mod api_client; // re-exported relevant types
mod api_error; // re-exported relevant types
mod parsed_url; // re-exported relevant types
mod uuid; // re-exported relevant types

pub mod application_passwords;
pub mod categories;
pub mod comments;
pub mod login;
pub mod media;
pub mod plugins;
pub mod post_types;
pub mod posts;
pub mod request;
pub mod site_settings;
pub mod tags;
pub mod taxonomies;
pub mod url_query;
pub mod users;
pub mod wordpress_org;
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

impl FromStr for WpApiParamOrder {
    type Err = EnumFromStrParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "asc" => Ok(Self::Asc),
            "desc" => Ok(Self::Desc),
            value => Err(EnumFromStrParsingError::UnknownVariant {
                value: value.to_string(),
            }),
        }
    }
}

trait SparseField {
    fn as_str(&self) -> &str;
}

trait OptionFromStr {
    type Err;

    fn option_from_str(s: &str) -> Result<Option<Self>, Self::Err>
    where
        Self: Sized;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, thiserror::Error)]
pub enum EnumFromStrParsingError {
    #[error("'{}' is not a valid variant for this enum", value)]
    UnknownVariant { value: String },
}

#[derive(Debug, Serialize, Deserialize, uniffi::Enum)]
#[serde(untagged)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

uniffi::custom_newtype!(WpResponseString, Option<String>);
#[derive(Debug, Serialize, Deserialize)]
#[serde(try_from = "BoolOrString")]
pub struct WpResponseString(pub Option<String>);

// In some cases, WordPress API may return a different type for a field than expected. One example,
// is when a `false` boolean value is returned when a `String` is expected.
//
// We handle these issues by deserializing them into some expected combinations, such as
// `BoolOrString` and then map them into a new type that wraps the original type. For example,
// `WpResponseString` is a new type for `Option<String>`, that uses `BoolOrString` to deserialize.
//
// During this conversion, there may be some values that are not clear how they should be mapped.
// For example, when we are expecting a `String` field, if we get a `false` value, we can assume
// that it's `null`, but there isn't a clear conversion for the `true` value, so we return an
// error.
#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum WpApiNewtypeParsingError {
    #[error("Expecting a `String` value for this field, but received the boolean `true` instead")]
    BooleanTrueIsReturnedWhenStringIsExpected,
}

impl TryFrom<BoolOrString> for WpResponseString {
    type Error = WpApiNewtypeParsingError;

    fn try_from(value: BoolOrString) -> Result<Self, Self::Error> {
        match value {
            BoolOrString::Bool(b) => {
                // When we are expecting a `String`, we can assume `false` means `null`, but there
                // isn't a clear conversion for `true`, so we return an error.
                if b {
                    Err(WpApiNewtypeParsingError::BooleanTrueIsReturnedWhenStringIsExpected)
                } else {
                    Ok(Self(None))
                }
            }
            BoolOrString::String(s) => Ok(Self(Some(s))),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Enum)]
#[serde(untagged)]
pub enum BoolOrString {
    Bool(bool),
    String(String),
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
