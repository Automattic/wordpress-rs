use serde::Serialize;
use std::collections::HashMap;
use wp_derive::WpDeserialize;

const EMPTY_ARRAY: &str = "[]";

#[derive(Serialize, WpDeserialize)]
#[serde(transparent)]
pub struct MapWrapper {
    pub items: Option<HashMap<String, String>>,
}

fn main() {
    // Empty array → None
    let result = serde_json::from_str::<MapWrapper>(EMPTY_ARRAY)
        .expect("MapWrapper should handle empty array");
    assert!(result.items.is_none());

    // JSON object → Some(HashMap)
    let result = serde_json::from_str::<MapWrapper>(r#"{"key": "value"}"#)
        .expect("MapWrapper should handle JSON object");
    let items = result.items.expect("items should be Some");
    assert_eq!(items.get("key"), Some(&"value".to_string()));
}
