use plugins::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use users::*;
use wp_localization::{MessageBundle, WpMessages, WpSupportsLocalization};
use wp_localization_macro::WpDeriveLocalizable;

pub mod jetpack;
pub mod wp_com;

pub mod api_client;
pub mod api_error;
pub mod application_passwords;
pub mod auth;
pub mod categories;
pub mod comments;
pub mod date;
pub mod login;
pub mod media;
pub mod middleware;
pub mod parsed_url;
pub mod plugins;
pub mod post_revisions;
pub mod post_types;
pub mod posts;
pub mod prelude;
pub mod request;
pub mod search_results;
pub mod site_settings;
pub mod ssl;
pub mod tags;
pub mod taxonomies;
pub mod templates;
pub mod themes;
pub mod url_query;
pub mod users;
pub mod uuid;
pub mod widget_types;
pub mod widgets;
pub mod wordpress_org;
pub mod wp_site_health_tests;

mod uniffi_serde;

#[cfg(feature = "reqwest-request-executor")]
pub mod reqwest_request_executor;

#[cfg(test)]
mod unit_test_common;

pub const WORDPRESS_RS_VERSION: &str = env!("CARGO_PKG_VERSION");

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

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum WpApiParamOrder {
    #[default]
    Asc,
    Desc,
}

impl_as_query_value_from_to_string!(WpApiParamOrder);

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

#[derive(Debug, PartialEq, Serialize, Deserialize, uniffi::Enum)]
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

/// Similar to `JsonValue`, but exported as a Uniffi object.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, uniffi::Object)]
#[uniffi::export(Eq, Hash)]
pub struct AnyJson {
    #[serde(flatten)]
    pub raw: Value,
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
#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error, WpDeriveLocalizable)]
pub enum WpApiNewtypeParsingError {
    BooleanTrueIsReturnedWhenStringIsExpected,
}

impl WpSupportsLocalization for WpApiNewtypeParsingError {
    fn message_bundle(&self) -> MessageBundle {
        match self {
            WpApiNewtypeParsingError::BooleanTrueIsReturnedWhenStringIsExpected => {
                WpMessages::boolean_true_is_returned_when_string_is_expected()
            }
        }
    }
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, uniffi::Enum)]
#[serde(untagged)]
pub enum BoolOrString {
    Bool(bool),
    String(String),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, uniffi::Enum)]
#[serde(untagged)]
pub enum BoolOrVecString {
    Bool(bool),
    VecString(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, uniffi::Enum)]
#[serde(untagged)]
pub enum IntegerOrString {
    Integer(i64),
    String(String),
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait WpAppNotifier: Send + Sync + std::fmt::Debug {
    async fn requested_with_invalid_authentication(&self);
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use url::Url;
    use url_query::QueryPairsExtension;

    #[rstest]
    #[case(WpApiParamOrder::Asc, "asc")]
    #[case(WpApiParamOrder::Desc, "desc")]
    fn test_order_url_query(#[case] orderby: WpApiParamOrder, #[case] expected: &str) {
        let mut url = Url::parse("https://example.com").unwrap();
        url.query_pairs_mut()
            .append_query_value_pair("orderby", &orderby);
        assert_eq!(
            url.query().map(|x| x.to_string()),
            Some(format!("orderby={expected}"))
        );
    }

    #[rstest]
    #[case(WpApiParamOrder::Asc)]
    #[case(WpApiParamOrder::Desc)]
    fn test_orderby_string_conversion(#[case] orderby: WpApiParamOrder) {
        assert_eq!(orderby, orderby.to_string().parse().unwrap());
    }

    #[derive(Deserialize, Debug)]
    struct Person {
        name: String,
        #[serde(flatten)]
        other_fields: AnyJson,
    }

    #[test]
    fn test_parse_any_json() {
        let json = r#"{"name": "Alice", "age": 30, "city": "Wonderland"}"#;
        let person: Person = serde_json::from_str(json).unwrap();
        assert_eq!(person.name, "Alice");
        assert_eq!(
            person.other_fields.raw,
            serde_json::json!({"age": 30, "city": "Wonderland"})
        );
    }

    #[test]
    fn test_parse_empty_any_json() {
        let json = r#"{"name": "Alice"}"#;
        let person: Person = serde_json::from_str(json).unwrap();
        assert_eq!(person.name, "Alice");
        assert_eq!(person.other_fields.raw, serde_json::json!({}));
    }
}
