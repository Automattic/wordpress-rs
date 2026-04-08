use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;

use crate::{
    post_types::PostType,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
};

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum TaxonomyType {
    Category,
    NavMenu,
    PostTag,
    WpPatternCategory,
    #[serde(untagged)]
    Custom(String),
}

impl Display for TaxonomyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Category => "category",
            Self::PostTag => "post_tag",
            Self::NavMenu => "nav_menu",
            Self::WpPatternCategory => "wp_pattern_category",
            Self::Custom(name) => name.as_str(),
        };
        write!(f, "{s}")
    }
}

impl From<&str> for TaxonomyType {
    fn from(s: &str) -> Self {
        match s {
            "category" => Self::Category,
            "post_tag" => Self::PostTag,
            "nav_menu" => Self::NavMenu,
            "wp_pattern_category" => Self::WpPatternCategory,
            _ => Self::Custom(s.to_string()),
        }
    }
}

impl From<String> for TaxonomyType {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum TaxonomyTypeCapabilities {
    AssignTerms,
    DeleteTerms,
    EditTerms,
    ManageTerms,
    #[serde(untagged)]
    Custom(String),
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum TaxonomyTypeLabels {
    AddNewItem,
    AddOrRemoveItems,
    AllItems,
    BackToItems,
    ChooseFromMostUsed,
    DescFieldDescription,
    EditItem,
    FilterByItem,
    ItemLink,
    ItemLinkDescription,
    ItemsList,
    ItemsListNavigation,
    MenuName,
    MostUsed,
    Name,
    NameAdminBar,
    NameFieldDescription,
    NewItemName,
    NoTerms,
    NotFound,
    ParentFieldDescription,
    ParentItem,
    ParentItemColon,
    PopularItems,
    SearchItems,
    SeparateItemsWithCommas,
    SingularName,
    SlugFieldDescription,
    TemplateName,
    UpdateItem,
    ViewItem,
    #[serde(untagged)]
    Custom(String),
}

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)]
pub struct TaxonomyListParams {
    /// Limit results to taxonomies associated with a specific post type.
    #[uniffi(default = None)]
    #[field_name("type")]
    pub post_type: Option<PostType>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseTaxonomyTypesResponse {
    #[serde(flatten)]
    #[WpContext(edit, embed, view)]
    #[WpContextualField]
    pub taxonomy_types: Option<HashMap<TaxonomyType, SparseTaxonomyTypeDetails>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseTaxonomyTypeDetails {
    #[WpContext(edit)]
    pub capabilities: Option<HashMap<TaxonomyTypeCapabilities, String>>,
    #[WpContext(edit, view)]
    pub description: Option<String>,
    #[WpContext(edit, view)]
    pub hierarchical: Option<bool>,
    #[WpContext(edit)]
    pub labels: Option<HashMap<TaxonomyTypeLabels, Option<String>>>,
    #[WpContext(edit, embed, view)]
    pub name: Option<String>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit)]
    pub show_cloud: Option<bool>,
    #[WpContext(edit, view)]
    pub types: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    pub rest_base: Option<String>,
    #[WpContext(edit, embed, view)]
    pub rest_namespace: Option<String>,
    #[WpContext(edit)]
    pub visibility: Option<TaxonomyTypeVisibility>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct TaxonomyTypeVisibility {
    pub public: bool,
    pub publicly_queryable: bool,
    pub show_admin_column: bool,
    pub show_in_nav_menus: bool,
    pub show_in_quick_edit: bool,
    pub show_ui: bool,
}
