use crate::{
    impl_as_query_value_from_as_str,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    AsQueryValue, BoolOrVecString, EnumFromStrParsingError,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display, str::FromStr};
use strum_macros::IntoStaticStr;
use wp_contextual::WpContextual;

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum ThemeStatus {
    Active,
    Inactive,
    #[serde(untagged)]
    Custom(String),
}

impl_as_query_value_from_as_str!(ThemeStatus);

impl ThemeStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Custom(status) => status,
        }
    }
}

impl FromStr for ThemeStatus {
    type Err = EnumFromStrParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            value => Ok(Self::Custom(value.to_string())),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct ThemeListParams {
    /// Limit result set to themes assigned one or more statuses.
    #[uniffi(default = None)]
    pub status: Option<ThemeStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, IntoStaticStr)]
enum ThemeListParamsField {
    #[strum(serialize = "status")]
    Status,
}

impl AppendUrlQueryPairs for ThemeListParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair(ThemeListParamsField::Status, self.status.as_ref());
    }
}

impl FromUrlQueryPairs for ThemeListParams {
    fn from_url_query_pairs(query_pairs: UrlQueryPairsMap) -> Option<Self> {
        Some(Self {
            status: query_pairs.get(ThemeListParamsField::Status),
        })
    }

    fn supports_pagination() -> bool {
        true
    }
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

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct ThemeAuthor {
    pub raw: String,
    pub rendered: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct ThemeAuthorName {
    pub raw: String,
    pub rendered: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct ThemeAuthorUri {
    pub raw: String,
    pub rendered: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct ThemeAuthorDescription {
    pub raw: String,
    pub rendered: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct ThemeDescription {
    pub raw: String,
    pub rendered: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct ThemeName {
    pub raw: String,
    pub rendered: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct ThemeTags {
    pub raw: Vec<String>,
    pub rendered: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
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
