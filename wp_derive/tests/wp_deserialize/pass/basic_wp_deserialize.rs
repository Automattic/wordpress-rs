use serde::Serialize;
use wp_derive::WpDeserialize;

#[derive(Serialize, WpDeserialize)]
#[serde(rename_all = "snake_case")]
pub struct SparseFoo {
    #[serde(rename = "bbar")]
    pub bar: Option<u32>,
}

fn main() {
    // Validate that "[]" is parsed successfully
    let result = serde_json::from_str::<SparseFoo>("[]");
    assert!(result.unwrap().bar.is_none());

    // Validate that `serde(rename = "bbar")` attribute is preserved
    let json = r#"{"bbar": 2}"#;
    let result = serde_json::from_str::<SparseFoo>(json);
    assert_eq!(result.unwrap().bar, Some(2));
}
