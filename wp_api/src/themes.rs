use crate::{
    BoolOrVecString, impl_as_query_value_from_to_string,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display};
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ThemeStatus {
    Active,
    Inactive,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

impl_as_query_value_from_to_string!(ThemeStatus);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct ThemeListParams {
    /// Limit result set to themes assigned one or more statuses.
    #[uniffi(default = None)]
    pub status: Option<ThemeStatus>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseTheme {
    #[WpContext(edit, embed, view)]
    pub stylesheet: Option<ThemeStylesheet>,
    #[WpContext(edit, embed, view)]
    pub template: Option<String>,
    #[WpContext(edit, embed, view)]
    pub requires_php: Option<String>,
    #[WpContext(edit, embed, view)]
    pub requires_wp: Option<String>,
    #[WpContext(edit, embed, view)]
    pub textdomain: Option<String>,
    #[WpContext(edit, embed, view)]
    pub version: Option<String>,
    #[WpContext(edit, embed, view)]
    pub screenshot: Option<String>,
    #[WpContext(edit, embed, view)]
    pub author: Option<ThemeAuthor>,
    #[WpContext(edit, embed, view)]
    pub author_uri: Option<ThemeAuthorUri>,
    #[WpContext(edit, embed, view)]
    pub description: Option<ThemeDescription>,
    #[WpContext(edit, embed, view)]
    pub name: Option<ThemeName>,
    #[WpContext(edit, embed, view)]
    pub tags: Option<ThemeTags>,
    #[WpContext(edit, embed, view)]
    pub theme_uri: Option<ThemeUri>,
    #[WpContext(edit, embed, view)]
    pub status: Option<ThemeStatus>,
    #[WpContext(edit, embed, view)]
    pub is_block_theme: Option<bool>,
    #[WpContext(edit, embed, view)]
    pub stylesheet_uri: Option<String>,
    #[WpContext(edit, embed, view)]
    pub template_uri: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub theme_supports: Option<HashMap<ThemeSupports, BoolOrVecString>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, uniffi::Record)]
#[serde(transparent)]
pub struct ThemeStylesheet {
    pub value: String,
}

impl ThemeStylesheet {
    pub fn new(value: String) -> Self {
        Self { value }
    }
}

impl From<&str> for ThemeStylesheet {
    fn from(value: &str) -> Self {
        Self {
            value: value.to_string(),
        }
    }
}

impl Display for ThemeStylesheet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ThemeAuthor {
    pub raw: String,
    pub rendered: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ThemeAuthorName {
    pub raw: String,
    pub rendered: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ThemeAuthorUri {
    pub raw: String,
    pub rendered: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ThemeAuthorDescription {
    pub raw: String,
    pub rendered: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ThemeDescription {
    pub raw: String,
    pub rendered: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ThemeName {
    pub raw: String,
    pub rendered: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ThemeTags {
    pub raw: Vec<String>,
    pub rendered: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ThemeUri {
    pub raw: String,
    pub rendered: String,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeSupports {
    AlignWide,
    AutomaticFeedLinks,
    BlockTemplates,
    BlockTemplateParts,
    CustomBackground,
    CustomHeader,
    CustomLogo,
    CustomizeSelectiveRefreshWidgets,
    DarkEditorStyle,
    DisableCustomColors,
    DisableCustomFontSizes,
    DisableCustomGradients,
    DisableLayoutStyles,
    EditorColorPalette,
    EditorFontSizes,
    EditorGradientPresets,
    EditorSpacingSizes,
    EditorStyles,
    Html5,
    Formats,
    PostThumbnails,
    ResponsiveEmbeds,
    TitleTag,
    WpBlockStyles,
    #[serde(untagged)]
    Custom(String),
}
