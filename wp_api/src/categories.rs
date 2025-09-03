use crate::{
    WpApiParamOrder, impl_as_query_value_from_to_string,
    posts::PostId,
    taxonomies::TaxonomyType,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_i64_id,
};
use serde::{Deserialize, Serialize};
use strum_macros::IntoStaticStr;
use wp_contextual::WpContextual;

wp_content_i64_id!(CategoryId);

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
pub enum WpApiParamCategoriesOrderBy {
    Id,
    Include,
    #[default]
    Name,
    Slug,
    IncludeSlugs,
    TermGroup,
    Description,
    Count,
}

impl_as_query_value_from_to_string!(WpApiParamCategoriesOrderBy);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct CategoryListParams {
    /// Current page of the collection.
    /// Default: `1`
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// Maximum number of items to be returned in result set.
    /// Default: `10`
    #[uniffi(default = None)]
    pub per_page: Option<u32>,
    /// Limit results to those matching a string.
    #[uniffi(default = None)]
    pub search: Option<String>,
    /// Ensure result set excludes specific IDs.
    #[uniffi(default = [])]
    pub exclude: Vec<CategoryId>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<CategoryId>,
    /// Offset the result set by a specific number of items.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// Order sort attribute ascending or descending.
    /// Default: `asc`
    /// One of: `asc`, `desc`
    #[uniffi(default = None)]
    pub order: Option<WpApiParamOrder>,
    /// Sort collection by user attribute.
    /// Default: `name`
    /// One of: `id`, `include`, `name`, `slug`, `include_slugs`, `term_group`, `description`, `count`
    #[uniffi(default = None)]
    pub orderby: Option<WpApiParamCategoriesOrderBy>,
    /// Whether to hide terms not assigned to any posts.
    #[uniffi(default = None)]
    pub hide_empty: Option<bool>,
    /// Limit result set to terms assigned to a specific parent.
    #[uniffi(default = None)]
    pub parent: Option<CategoryId>,
    /// Limit result set to terms assigned to a specific post.
    #[uniffi(default = None)]
    pub post: Option<PostId>,
    /// Limit result set to users with one or more specific slugs.
    #[uniffi(default = [])]
    pub slug: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, IntoStaticStr)]
enum CategoryListParamsField {
    #[strum(serialize = "page")]
    Page,
    #[strum(serialize = "per_page")]
    PerPage,
    #[strum(serialize = "search")]
    Search,
    #[strum(serialize = "exclude")]
    Exclude,
    #[strum(serialize = "include")]
    Include,
    #[strum(serialize = "offset")]
    Offset,
    #[strum(serialize = "order")]
    Order,
    #[strum(serialize = "orderby")]
    Orderby,
    #[strum(serialize = "hide_empty")]
    HideEmpty,
    #[strum(serialize = "parent")]
    Parent,
    #[strum(serialize = "post")]
    Post,
    #[strum(serialize = "slug")]
    Slug,
}

impl AppendUrlQueryPairs for CategoryListParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair(CategoryListParamsField::Page, self.page.as_ref())
            .append_option_query_value_pair(
                CategoryListParamsField::PerPage,
                self.per_page.as_ref(),
            )
            .append_option_query_value_pair(CategoryListParamsField::Search, self.search.as_ref())
            .append_vec_query_value_pair(CategoryListParamsField::Exclude, &self.exclude)
            .append_vec_query_value_pair(CategoryListParamsField::Include, &self.include)
            .append_option_query_value_pair(CategoryListParamsField::Offset, self.offset.as_ref())
            .append_option_query_value_pair(CategoryListParamsField::Order, self.order.as_ref())
            .append_option_query_value_pair(CategoryListParamsField::Orderby, self.orderby.as_ref())
            .append_option_query_value_pair(
                CategoryListParamsField::HideEmpty,
                self.hide_empty.as_ref(),
            )
            .append_option_query_value_pair(CategoryListParamsField::Parent, self.parent.as_ref())
            .append_option_query_value_pair(CategoryListParamsField::Post, self.post.as_ref())
            .append_vec_query_value_pair(CategoryListParamsField::Slug, &self.slug);
    }
}

impl FromUrlQueryPairs for CategoryListParams {
    fn from_url_query_pairs(query_pairs: UrlQueryPairsMap) -> Option<Self> {
        Some(Self {
            page: query_pairs.get(CategoryListParamsField::Page),
            per_page: query_pairs.get(CategoryListParamsField::PerPage),
            search: query_pairs.get(CategoryListParamsField::Search),
            exclude: query_pairs.get_csv(CategoryListParamsField::Exclude),
            include: query_pairs.get_csv(CategoryListParamsField::Include),
            offset: query_pairs.get(CategoryListParamsField::Offset),
            order: query_pairs.get(CategoryListParamsField::Order),
            orderby: query_pairs.get(CategoryListParamsField::Orderby),
            hide_empty: query_pairs.get(CategoryListParamsField::HideEmpty),
            parent: query_pairs.get(CategoryListParamsField::Parent),
            post: query_pairs.get(CategoryListParamsField::Post),
            slug: query_pairs.get_csv(CategoryListParamsField::Slug),
        })
    }

    fn supports_pagination() -> bool {
        true
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct CategoryDeleteResponse {
    pub deleted: bool,
    pub previous: CategoryWithEditContext,
}

#[derive(Debug, Serialize, uniffi::Record)]
pub struct CategoryCreateParams {
    /// HTML title for the term.
    pub name: String,
    /// HTML description of the term.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// An alphanumeric identifier for the term unique to its type.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// The parent term ID.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<CategoryId>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/470
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct CategoryUpdateParams {
    /// HTML title for the term.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// HTML description of the term.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// An alphanumeric identifier for the term unique to its type.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// The parent term ID.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<CategoryId>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/470
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseCategory {
    #[WpContext(edit, embed, view)]
    pub id: Option<CategoryId>,
    #[WpContext(edit, view)]
    pub count: Option<i64>,
    #[WpContext(edit, view)]
    pub description: Option<String>,
    #[WpContext(edit, embed, view)]
    pub link: Option<String>,
    #[WpContext(edit, embed, view)]
    pub name: Option<String>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit, embed, view)]
    pub taxonomy: Option<TaxonomyType>,
    #[WpContext(edit, view)]
    pub parent: Option<CategoryId>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/470
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use url::Url;

    #[rstest]
    #[case(WpApiParamCategoriesOrderBy::Id, "id")]
    #[case(WpApiParamCategoriesOrderBy::Include, "include")]
    #[case(WpApiParamCategoriesOrderBy::Name, "name")]
    #[case(WpApiParamCategoriesOrderBy::Slug, "slug")]
    #[case(WpApiParamCategoriesOrderBy::IncludeSlugs, "include_slugs")]
    #[case(WpApiParamCategoriesOrderBy::TermGroup, "term_group")]
    #[case(WpApiParamCategoriesOrderBy::Description, "description")]
    #[case(WpApiParamCategoriesOrderBy::Count, "count")]
    fn test_orderby_url_query(
        #[case] orderby: WpApiParamCategoriesOrderBy,
        #[case] expected: &str,
    ) {
        let mut url = Url::parse("https://example.com").unwrap();
        url.query_pairs_mut()
            .append_query_value_pair("orderby", &orderby);
        assert_eq!(
            url.query().map(|x| x.to_string()),
            Some(format!("orderby={expected}"))
        );
    }

    #[rstest]
    #[case(WpApiParamCategoriesOrderBy::Id)]
    #[case(WpApiParamCategoriesOrderBy::Include)]
    #[case(WpApiParamCategoriesOrderBy::Name)]
    #[case(WpApiParamCategoriesOrderBy::Slug)]
    #[case(WpApiParamCategoriesOrderBy::IncludeSlugs)]
    #[case(WpApiParamCategoriesOrderBy::TermGroup)]
    #[case(WpApiParamCategoriesOrderBy::Description)]
    #[case(WpApiParamCategoriesOrderBy::Count)]
    fn test_orderby_string_conversion(#[case] orderby: WpApiParamCategoriesOrderBy) {
        assert_eq!(orderby, orderby.to_string().parse().unwrap());
    }
}
