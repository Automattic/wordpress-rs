use plugins::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use users::*;
use wp_localization::{MessageBundle, WpMessages, WpSupportsLocalization};
use wp_localization_macro::WpDeriveLocalizable;

pub mod jetpack;
pub mod wp_com;

pub mod api_client;
pub mod api_error;
pub mod application_passwords;
pub mod auth;
pub mod block_directory;
pub mod block_pattern_categories;
pub mod block_patterns;
pub mod block_renderer;
pub mod block_types;
pub mod comments;
pub mod date;
pub mod decimal2;
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
pub mod template_autosaves;
pub mod template_part_autosaves;
pub mod template_part_revisions;
pub mod template_parts;
pub mod template_revisions;
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
            Value::Object(obj) => JsonValue::Object(
                obj.iter()
                    .map(|(k, v)| (k.clone(), JsonValue::from(v)))
                    .collect(),
            ),
        }
    }
}

impl From<JsonValue> for Value {
    fn from(value: JsonValue) -> Self {
        match value {
            JsonValue::Null => Value::Null,
            JsonValue::Bool(b) => Value::Bool(b),
            JsonValue::Int(i) => Value::Number(i.into()),
            // serde_json::Number::from_f64 returns None for NaN/Infinity.
            // JsonValue does not distinguish those from regular floats; if
            // we ever produce a non-finite float we serialize it as Null
            // rather than panic. WordPress meta does not produce non-finite
            // numbers in practice.
            JsonValue::Float(f) => serde_json::Number::from_f64(f)
                .map(Value::Number)
                .unwrap_or(Value::Null),
            JsonValue::String(s) => Value::String(s),
            JsonValue::Array(arr) => Value::Array(arr.into_iter().map(Value::from).collect()),
            JsonValue::Object(map) => {
                Value::Object(map.into_iter().map(|(k, v)| (k, Value::from(v))).collect())
            }
        }
    }
}

/// Type-erased JSON object used to capture or inject WordPress's
/// `additional_fields` extension surface — the plugin-registered REST fields
/// that aren't part of any typed response. Exported as a Uniffi object so
/// Swift/Kotlin consumers can read or build these payloads.
///
/// The `#[serde(flatten)]` on `raw` is intentional: this type is meant to be
/// embedded as `#[serde(flatten)] pub additional_fields: WpAdditionalFields` on
/// a host response struct, so that unknown keys land here while typed keys
/// stay on the host.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, uniffi::Object)]
#[uniffi::export(Eq, Hash)]
pub struct WpAdditionalFields {
    #[serde(flatten)]
    pub raw: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum AdditionalFieldsParseError {
    #[error("Failed to parse JSON: {reason}")]
    InvalidJson { reason: String },
}

#[uniffi::export]
impl WpAdditionalFields {
    /// Constructs an empty `WpAdditionalFields` with no keys held. The serialized
    /// form is `{}`. Designed to be the starting point for a builder chain
    /// (`WpAdditionalFields::new().with_value(...)`).
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(WpAdditionalFields {
            raw: Value::Object(serde_json::Map::new()),
        })
    }

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

    /// Returns the value at `key` as a `String` only when the underlying JSON
    /// value is a string. Numbers, booleans, arrays, objects, and `null` all
    /// yield `None` — there is no coercion (e.g. `42` does not become `"42"`).
    pub fn string_value_for_key(&self, key: &str) -> Option<String> {
        match self.raw.get(key) {
            Some(Value::String(s)) => Some(s.clone()),
            _ => None,
        }
    }

    /// Returns the value at `key` as a `bool` only when the underlying JSON
    /// value is a boolean. No coercion from strings or numbers.
    pub fn bool_value_for_key(&self, key: &str) -> Option<bool> {
        match self.raw.get(key) {
            Some(Value::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    /// Returns the value at `key` as an array of `JsonValue` only when the
    /// underlying JSON value is an array.
    pub fn array_value_for_key(&self, key: &str) -> Option<Vec<JsonValue>> {
        match self.raw.get(key) {
            Some(Value::Array(arr)) => Some(arr.iter().map(JsonValue::from).collect()),
            _ => None,
        }
    }

    /// Returns the value at `key` as an object (string-keyed map of
    /// `JsonValue`) only when the underlying JSON value is an object.
    pub fn object_value_for_key(&self, key: &str) -> Option<HashMap<String, JsonValue>> {
        match self.raw.get(key) {
            Some(Value::Object(map)) => Some(
                map.iter()
                    .map(|(k, v)| (k.clone(), JsonValue::from(v)))
                    .collect(),
            ),
            _ => None,
        }
    }

    /// Creates a `WpAdditionalFields` from a map of keys to term ID arrays.
    /// Used to construct the additional_fields for PostCreateParams
    /// and PostUpdateParams with custom taxonomy term IDs.
    #[uniffi::constructor]
    pub fn from_term_id_map(map: HashMap<String, Vec<terms::TermId>>) -> Arc<Self> {
        let mut json_map = serde_json::Map::new();
        for (key, ids) in map {
            let arr: Vec<Value> = ids
                .into_iter()
                .map(|id| Value::Number(id.0.into()))
                .collect();
            json_map.insert(key, Value::Array(arr));
        }
        Arc::new(WpAdditionalFields {
            raw: Value::Object(json_map),
        })
    }

    /// Parses a JSON string into a `WpAdditionalFields`. Used to construct arbitrary
    /// plugin fields (e.g. Publicize connections + message) for injection
    /// into `PostCreateParams.additional_fields`.
    #[uniffi::constructor]
    pub fn from_json_string(json: String) -> Result<Arc<Self>, AdditionalFieldsParseError> {
        let value: Value =
            serde_json::from_str(&json).map_err(|e| AdditionalFieldsParseError::InvalidJson {
                reason: e.to_string(),
            })?;
        Ok(Arc::new(WpAdditionalFields { raw: value }))
    }

    /// Returns a new `WpAdditionalFields` with `key` set to `value`. Receiver
    /// `Arc` is not mutated; callers chain:
    /// `WpAdditionalFields::new().with_value("k".into(), JsonValue::String("v".into()))`.
    ///
    /// Defensive recovery: if `raw` is not a JSON object — which can only
    /// happen when this `WpAdditionalFields` was deserialized from a non-object
    /// payload — the result is a fresh object containing only `(key, value)`,
    /// so consumers can rely on the result always being object-typed.
    pub fn with_value(self: Arc<Self>, key: String, value: JsonValue) -> Arc<Self> {
        let mut new_map = match &self.raw {
            Value::Object(map) => map.clone(),
            _ => serde_json::Map::new(),
        };
        new_map.insert(key, value.into());
        Arc::new(WpAdditionalFields {
            raw: Value::Object(new_map),
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
        other_fields: WpAdditionalFields,
    }

    #[test]
    fn test_parse_additional_fields() {
        let json = r#"{"name": "Alice", "age": 30, "city": "Wonderland"}"#;
        let person: Person = serde_json::from_str(json).unwrap();
        assert_eq!(person.name, "Alice");
        assert_eq!(
            person.other_fields.raw,
            serde_json::json!({"age": 30, "city": "Wonderland"})
        );
    }

    #[test]
    fn test_parse_empty_additional_fields() {
        let json = r#"{"name": "Alice"}"#;
        let person: Person = serde_json::from_str(json).unwrap();
        assert_eq!(person.name, "Alice");
        assert_eq!(person.other_fields.raw, serde_json::json!({}));
    }

    #[test]
    fn test_json_value_from_null() {
        assert_eq!(JsonValue::from(&serde_json::json!(null)), JsonValue::Null);
    }

    #[test]
    fn test_json_value_from_bool() {
        assert_eq!(
            JsonValue::from(&serde_json::json!(true)),
            JsonValue::Bool(true)
        );
        assert_eq!(
            JsonValue::from(&serde_json::json!(false)),
            JsonValue::Bool(false)
        );
    }

    #[test]
    fn test_json_value_from_positive_int() {
        assert_eq!(JsonValue::from(&serde_json::json!(42)), JsonValue::Int(42));
    }

    #[test]
    fn test_json_value_from_negative_int() {
        assert_eq!(JsonValue::from(&serde_json::json!(-7)), JsonValue::Int(-7));
    }

    #[test]
    fn test_json_value_from_large_u64() {
        // u64::MAX doesn't fit in i64, so it falls through to the u64 → f64 path
        let val = serde_json::json!(u64::MAX);
        assert_eq!(JsonValue::from(&val), JsonValue::Float(u64::MAX as f64));
    }

    #[test]
    fn test_json_value_from_float() {
        assert_eq!(
            JsonValue::from(&serde_json::json!(2.5)),
            JsonValue::Float(2.5)
        );
    }

    #[test]
    fn test_json_value_from_string() {
        assert_eq!(
            JsonValue::from(&serde_json::json!("hello")),
            JsonValue::String("hello".to_string())
        );
    }

    #[test]
    fn test_json_value_from_array() {
        assert_eq!(
            JsonValue::from(&serde_json::json!([1, "two", null])),
            JsonValue::Array(vec![
                JsonValue::Int(1),
                JsonValue::String("two".to_string()),
                JsonValue::Null,
            ])
        );
    }

    #[test]
    fn test_json_value_from_nested_object() {
        assert_eq!(
            JsonValue::from(&serde_json::json!({"a": {"b": [1, 2]}})),
            JsonValue::Object(HashMap::from([(
                "a".to_string(),
                JsonValue::Object(HashMap::from([(
                    "b".to_string(),
                    JsonValue::Array(vec![JsonValue::Int(1), JsonValue::Int(2)])
                )]))
            )]))
        );
    }
}

#[cfg(test)]
mod additional_fields_tests {
    use super::{JsonValue, WpAdditionalFields};
    use std::collections::HashMap;

    #[test]
    fn from_json_string_parses_object() {
        let json = WpAdditionalFields::from_json_string(r#"{"a":1,"b":"x"}"#.to_string()).unwrap();
        assert_eq!(json.value_for_key("a"), Some(JsonValue::Int(1)));
        assert_eq!(
            json.value_for_key("b"),
            Some(JsonValue::String("x".to_string()))
        );
    }

    #[test]
    fn from_json_string_rejects_invalid() {
        assert!(WpAdditionalFields::from_json_string("not-json".to_string()).is_err());
    }

    #[test]
    fn new_produces_empty_object() {
        let fields = WpAdditionalFields::new();
        assert!(fields.keys().is_empty());
        // Round-trips as `{}` rather than `null` so it's safe to merge into.
        assert_eq!(
            serde_json::to_value(&*fields).unwrap(),
            serde_json::json!({})
        );
    }

    #[test]
    fn with_value_inserts_key_into_empty_object() {
        let fields = WpAdditionalFields::new().with_value("k".to_string(), JsonValue::Int(42));
        assert_eq!(fields.value_for_key("k"), Some(JsonValue::Int(42)));
    }

    #[test]
    fn with_value_overwrites_existing_key() {
        let fields = WpAdditionalFields::new()
            .with_value("k".to_string(), JsonValue::String("first".to_string()))
            .with_value("k".to_string(), JsonValue::String("second".to_string()));
        assert_eq!(
            fields.value_for_key("k"),
            Some(JsonValue::String("second".to_string()))
        );
    }

    #[test]
    fn with_value_recovers_when_raw_is_not_object() {
        // A WpAdditionalFields deserialized from a non-object payload (e.g. a
        // bare number) should not panic when written to; with_value replaces
        // with a fresh object containing only the new key.
        let non_object = WpAdditionalFields::from_json_string("42".to_string()).unwrap();
        let fields = non_object.with_value("k".to_string(), JsonValue::String("v".to_string()));
        assert_eq!(fields.keys(), vec!["k".to_string()]);
        assert_eq!(
            fields.value_for_key("k"),
            Some(JsonValue::String("v".to_string()))
        );
    }

    #[test]
    fn string_value_for_key_returns_string() {
        let fields = WpAdditionalFields::new()
            .with_value("k".to_string(), JsonValue::String("hello".to_string()));
        assert_eq!(fields.string_value_for_key("k"), Some("hello".to_string()));
    }

    #[test]
    fn string_value_for_key_does_not_coerce_number() {
        // Explicit no-coercion contract: numeric values must not become strings.
        let fields = WpAdditionalFields::new().with_value("k".to_string(), JsonValue::Int(42));
        assert_eq!(fields.string_value_for_key("k"), None);
    }

    #[test]
    fn string_value_for_key_returns_none_for_missing_key() {
        let fields = WpAdditionalFields::new();
        assert_eq!(fields.string_value_for_key("missing"), None);
    }

    #[test]
    fn bool_value_for_key_returns_bool() {
        let fields =
            WpAdditionalFields::new().with_value("flag".to_string(), JsonValue::Bool(true));
        assert_eq!(fields.bool_value_for_key("flag"), Some(true));
    }

    #[test]
    fn bool_value_for_key_does_not_coerce_string() {
        let fields = WpAdditionalFields::new()
            .with_value("flag".to_string(), JsonValue::String("true".to_string()));
        assert_eq!(fields.bool_value_for_key("flag"), None);
    }

    #[test]
    fn array_value_for_key_returns_array() {
        let fields = WpAdditionalFields::new().with_value(
            "items".to_string(),
            JsonValue::Array(vec![JsonValue::Int(1), JsonValue::Int(2)]),
        );
        assert_eq!(
            fields.array_value_for_key("items"),
            Some(vec![JsonValue::Int(1), JsonValue::Int(2)])
        );
    }

    #[test]
    fn array_value_for_key_returns_none_for_object() {
        let fields = WpAdditionalFields::new().with_value(
            "obj".to_string(),
            JsonValue::Object(HashMap::from([("k".to_string(), JsonValue::Int(1))])),
        );
        assert_eq!(fields.array_value_for_key("obj"), None);
    }

    #[test]
    fn object_value_for_key_returns_object() {
        let inner = HashMap::from([("nested".to_string(), JsonValue::String("v".to_string()))]);
        let fields = WpAdditionalFields::new()
            .with_value("obj".to_string(), JsonValue::Object(inner.clone()));
        assert_eq!(fields.object_value_for_key("obj"), Some(inner));
    }

    #[test]
    fn object_value_for_key_returns_none_for_array() {
        let fields = WpAdditionalFields::new()
            .with_value("arr".to_string(), JsonValue::Array(vec![JsonValue::Int(1)]));
        assert_eq!(fields.object_value_for_key("arr"), None);
    }

    #[test]
    fn array_and_object_value_for_key_round_trip_complex_payload() {
        // Build a nested payload via from_json_string, then read it back via
        // the typed accessors to confirm the conversion preserves structure.
        let fields = WpAdditionalFields::from_json_string(
            r#"{
                "tags": ["a", "b", 3],
                "meta": {"author": "alice", "count": 7}
            }"#
            .to_string(),
        )
        .unwrap();

        let tags = fields.array_value_for_key("tags").unwrap();
        assert_eq!(
            tags,
            vec![
                JsonValue::String("a".to_string()),
                JsonValue::String("b".to_string()),
                JsonValue::Int(3),
            ]
        );

        let meta = fields.object_value_for_key("meta").unwrap();
        assert_eq!(
            meta.get("author"),
            Some(&JsonValue::String("alice".to_string()))
        );
        assert_eq!(meta.get("count"), Some(&JsonValue::Int(7)));
    }
}

#[cfg(test)]
mod json_value_tests {
    use super::*;
    use serde_json::Value;
    use std::collections::HashMap;

    #[test]
    fn json_value_round_trips_via_serde_json_value() {
        // Each variant: JsonValue -> Value -> JsonValue must equal original.
        let cases = vec![
            JsonValue::Null,
            JsonValue::Bool(true),
            JsonValue::Bool(false),
            JsonValue::Int(0),
            JsonValue::Int(-1),
            JsonValue::Int(i64::MAX),
            JsonValue::Float(1.5),
            JsonValue::String("hello".to_string()),
            JsonValue::Array(vec![JsonValue::Int(1), JsonValue::String("a".to_string())]),
            JsonValue::Object({
                let mut m = HashMap::new();
                m.insert("k".to_string(), JsonValue::Bool(true));
                m
            }),
        ];
        for original in cases {
            let as_value: Value = original.clone().into();
            let back: JsonValue = JsonValue::from(&as_value);
            assert_eq!(original, back, "round-trip failed for {:?}", original);
        }
    }

    #[test]
    fn float_nan_and_infinity_map_to_null() {
        // f64::NAN and f64::INFINITY have no JSON representation.
        // The From<JsonValue> for Value impl maps them to Value::Null
        // rather than panicking. Round-trip is not testable (NaN != NaN);
        // this is a one-way assertion.
        let nan: serde_json::Value = JsonValue::Float(f64::NAN).into();
        assert_eq!(nan, serde_json::Value::Null);

        let inf: serde_json::Value = JsonValue::Float(f64::INFINITY).into();
        assert_eq!(inf, serde_json::Value::Null);

        let neg_inf: serde_json::Value = JsonValue::Float(f64::NEG_INFINITY).into();
        assert_eq!(neg_inf, serde_json::Value::Null);
    }
}
