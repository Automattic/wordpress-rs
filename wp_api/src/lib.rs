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
pub mod comments;
pub mod date;
pub mod login;
pub mod media;
pub mod menu_locations;
pub mod middleware;
pub mod nav_menu_item_revisions;
pub mod nav_menu_items;
pub mod nav_menus;
pub mod navigation_revisions;
pub mod navigations;
pub mod parsed_url;
pub mod plugins;
pub mod post_revisions;
pub mod post_statuses;
pub mod post_types;
pub mod posts;
pub mod prelude;
pub mod request;
pub mod search_results;
pub mod site_settings;
pub mod ssl;
pub mod taxonomies;
pub mod templates;
pub mod terms;
pub mod themes;
pub mod url_query;
pub mod users;
pub mod uuid;
pub mod widget_types;
pub mod widgets;
pub mod wordpress_org;
pub mod wp_block_editor;
pub mod wp_content_macros;
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
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum WpApiParamOrder {
    #[serde(alias = "asc")]
    #[default]
    Asc,

    #[serde(alias = "desc")]
    Desc,
}

impl_as_query_value_from_to_string!(WpApiParamOrder);

trait SparseField {
    fn as_mapped_field_name(&self) -> &str;
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Enum)]
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

impl From<&Value> for JsonValue {
    fn from(value: &Value) -> Self {
        match value {
            Value::Null => JsonValue::Null,
            Value::Bool(b) => JsonValue::Bool(*b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    JsonValue::Int(i)
                } else if let Some(u) = n.as_u64() {
                    // u64 values that don't fit in i64 (> i64::MAX) — store as
                    // float, accepting precision loss beyond 2^53. WordPress
                    // APIs don't produce values in this range.
                    JsonValue::Float(u as f64)
                } else {
                    // Must be a float — as_f64() only returns None for
                    // out-of-range values which serde_json won't parse without
                    // the arbitrary_precision feature.
                    JsonValue::Float(n.as_f64().expect("unexpected unrepresentable JSON number"))
                }
            }
            Value::String(s) => JsonValue::String(s.clone()),
            Value::Array(arr) => JsonValue::Array(arr.iter().map(JsonValue::from).collect()),
            Value::Object(obj) => {
                JsonValue::Object(obj.iter().map(|(k, v)| (k.clone(), JsonValue::from(v))).collect())
            }
        }
    }
}

/// Similar to `JsonValue`, but exported as a Uniffi object.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, uniffi::Object)]
#[uniffi::export(Eq, Hash)]
pub struct AnyJson {
    #[serde(flatten)]
    pub raw: Value,
}

#[uniffi::export]
impl AnyJson {
    /// Extracts an array of term IDs from the additional fields for a given key.
    /// Returns an empty vec if the key doesn't exist or isn't an array of integers.
    pub fn term_ids_for_key(&self, key: &str) -> Vec<terms::TermId> {
        match self.raw.get(key) {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_i64().map(terms::TermId))
                .collect(),
            _ => vec![],
        }
    }

    /// Returns the keys present in this JSON object, or an empty vec if not an object.
    pub fn keys(&self) -> Vec<String> {
        match &self.raw {
            Value::Object(map) => map.keys().cloned().collect(),
            _ => vec![],
        }
    }

    /// Returns the value for a given key as a `JsonValue`, or `None` if the key
    /// doesn't exist or this isn't an object.
    pub fn value_for_key(&self, key: &str) -> Option<JsonValue> {
        self.raw.get(key).map(JsonValue::from)
    }

    /// Creates an AnyJson from a map of keys to term ID arrays.
    /// Used to construct the additional_fields for PostCreateParams
    /// and PostUpdateParams with custom taxonomy term IDs.
    #[uniffi::constructor]
    pub fn from_term_id_map(map: HashMap<String, Vec<terms::TermId>>) -> std::sync::Arc<Self> {
        let mut json_map = serde_json::Map::new();
        for (key, ids) in map {
            let arr: Vec<Value> = ids
                .into_iter()
                .map(|id| Value::Number(id.0.into()))
                .collect();
            json_map.insert(key, Value::Array(arr));
        }
        std::sync::Arc::new(AnyJson {
            raw: Value::Object(json_map),
        })
    }
}

uniffi::custom_newtype!(WpResponseString, Option<String>);
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    fn message_bundle(&self) -> MessageBundle<'_> {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, uniffi::Enum)]
#[serde(untagged)]
pub enum BoolOrString {
    Bool(bool),
    String(String),
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
    async fn requested_with_invalid_authentication(&self, request_url: String);
}

#[derive(Debug)]
pub struct EmptyAppNotifier;

#[async_trait::async_trait]
impl WpAppNotifier for EmptyAppNotifier {
    async fn requested_with_invalid_authentication(&self, _request_url: String) {
        // no-op
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

#[uniffi::export]
#[allow(unused_variables)] // The app_id is only used on Apple platforms.
pub fn setup_logger(app_id: String) {
    #[cfg(debug_assertions)]
    let log_level = log::LevelFilter::Debug;
    #[cfg(not(debug_assertions))]
    let log_level = log::LevelFilter::Warn;

    #[cfg(target_vendor = "apple")]
    {
        match oslog::OsLogger::new(&app_id).level_filter(log_level).init() {
            Ok(_) => println!("Logger initialized successfully."),
            Err(e) => println!("Failed to initialize logger: {e}"),
        }
    }

    #[cfg(target_os = "android")]
    {
        android_logger::init_once(android_logger::Config::default().with_max_level(log_level));
        println!("Logger initialized successfully.");
    }

    #[cfg(not(any(target_vendor = "apple", target_os = "android")))]
    {
        println!("Logger not configured for this platform.");
    }
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
