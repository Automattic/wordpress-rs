use crate::{JsonValue, wp_content_string_id};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wp_contextual::WpContextual;

wp_content_string_id!(BlockTypeNamespace);
wp_content_string_id!(BlockTypeName);

// Deprecated fields from the WP REST API schema are intentionally omitted:
// `editor_script`, `script`, `view_script`, `editor_style`, `style`
// — all replaced by their `_handles` counterparts.
#[derive(Debug, Serialize, Deserialize, WpContextual)]
pub struct SparseBlockType {
    #[WpContext(edit, embed, view)]
    pub api_version: Option<i64>,
    #[WpContext(edit, embed, view)]
    pub title: Option<String>,
    #[WpContext(edit, embed, view)]
    pub name: Option<String>,
    #[WpContext(edit, embed, view)]
    pub description: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub icon: Option<String>,
    // Block attributes schema — keys and structure vary per block type, so this
    // can't be a fixed struct. WordPress also returns `[]` for empty objects.
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    #[serde(
        default,
        deserialize_with = "wp_serde_helper::deserialize_option_empty_array_or_hashmap"
    )]
    pub attributes: Option<HashMap<String, JsonValue>>,
    // Maps context key names to block attribute names. WordPress returns `[]`
    // for empty objects.
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    #[serde(
        default,
        deserialize_with = "wp_serde_helper::deserialize_option_empty_array_or_hashmap"
    )]
    pub provides_context: Option<HashMap<String, String>>,
    #[WpContext(edit, embed, view)]
    pub uses_context: Option<Vec<String>>,
    // Custom CSS selectors — keys vary per block type. WordPress returns `[]`
    // for empty objects.
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    #[serde(
        default,
        deserialize_with = "wp_serde_helper::deserialize_option_empty_array_or_hashmap"
    )]
    pub selectors: Option<HashMap<String, JsonValue>>,
    // Block feature support flags — keys are feature names (e.g. "color",
    // "typography") with boolean or object values that vary per block type.
    // WordPress returns `[]` for empty objects.
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    #[serde(
        default,
        deserialize_with = "wp_serde_helper::deserialize_option_empty_array_or_hashmap"
    )]
    pub supports: Option<HashMap<String, JsonValue>>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub category: Option<String>,
    #[WpContext(edit, embed, view)]
    pub is_dynamic: Option<bool>,
    #[WpContext(edit, embed, view)]
    pub editor_script_handles: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    pub script_handles: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    pub view_script_handles: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    pub view_script_module_ids: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    pub editor_style_handles: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    pub style_handles: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    pub view_style_handles: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    pub styles: Option<Vec<BlockStyleVariation>>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub variations: Option<Vec<BlockVariation>>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub textdomain: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub parent: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub ancestor: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    pub keywords: Option<Vec<String>>,
    // The PHP schema defines `attributes` and `innerBlocks` as known properties,
    // but blocks can register arbitrary example data (e.g. `viewportWidth`), so
    // this can't be a fixed struct. WordPress returns `[]` for empty objects.
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    #[serde(
        default,
        deserialize_with = "wp_serde_helper::deserialize_option_empty_array_or_hashmap"
    )]
    pub example: Option<HashMap<String, JsonValue>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, uniffi::Record)]
pub struct BlockStyleVariation {
    pub name: String,
    pub label: Option<String>,
    pub inline_style: Option<String>,
    pub style_handle: Option<String>,
}

// `WpDeserialize` handles `[]` → all fields `None`, which WordPress returns for
// empty variations (e.g. `core/term-template` returns `"variations": [[]]`).
#[derive(Debug, Serialize, PartialEq, wp_derive::WpDeserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct BlockVariation {
    pub name: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub icon: Option<String>,
    pub is_default: Option<bool>,
    // Variation attribute overrides — keys vary per block type.
    pub attributes: Option<HashMap<String, JsonValue>>,
    pub inner_blocks: Option<Vec<JsonValue>>,
    // Same as the top-level `example` — blocks can register arbitrary keys.
    pub example: Option<HashMap<String, JsonValue>>,
    pub scope: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
}
