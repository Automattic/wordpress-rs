use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};
use strum_macros::IntoStaticStr;
use wp_contextual::WpContextual;

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

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct TaxonomyListParams {
    /// Limit results to taxonomies associated with a specific post type.
    #[uniffi(default = None)]
    pub post_type: Option<PostType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, IntoStaticStr)]
enum TaxonomyListParamsField {
    #[strum(serialize = "type")]
    PostType,
}

impl AppendUrlQueryPairs for TaxonomyListParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut.append_option_query_value_pair(
            TaxonomyListParamsField::PostType,
            self.post_type.as_ref(),
        );
    }
}

impl FromUrlQueryPairs for TaxonomyListParams {
    fn from_url_query_pairs(query_pairs: UrlQueryPairsMap) -> Option<Self> {
        Some(Self {
            post_type: query_pairs.get(TaxonomyListParamsField::PostType),
        })
    }

    fn supports_pagination() -> bool {
        false
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
#[serde(transparent)]
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
