//! Cache key generation for metadata collections.
//!
//! This module provides functions to generate deterministic cache keys from
//! API parameters. The cache key is used to identify unique list configurations
//! in the metadata store.

use url::Url;
use wp_api::{
    posts::{PostListParams, PostListParamsField},
    url_query::AsQueryValue,
};

/// Extension trait to add query pairs using `AsQueryValue`.
///
/// This replicates the functionality of `wp_api::url_query::QueryPairsExtension`
/// which is `pub(crate)` in wp_api.
trait QueryPairsExt {
    fn append_option<T: AsQueryValue>(&mut self, key: &str, value: Option<&T>);
    fn append_vec<T: AsQueryValue>(&mut self, key: &str, value: &[T]);
}

impl QueryPairsExt for url::form_urlencoded::Serializer<'_, url::UrlQuery<'_>> {
    fn append_option<T: AsQueryValue>(&mut self, key: &str, value: Option<&T>) {
        if let Some(v) = value {
            self.append_pair(key, v.as_query_value().as_ref());
        }
    }

    fn append_vec<T: AsQueryValue>(&mut self, key: &str, value: &[T]) {
        if !value.is_empty() {
            let csv: String = value
                .iter()
                .map(|v| v.as_query_value().as_ref().to_string())
                .collect::<Vec<_>>()
                .join(",");
            self.append_pair(key, &csv);
        }
    }
}

/// Generates a deterministic cache key from `PostListParams`.
///
/// This function explicitly includes only filter-relevant fields, excluding
/// pagination and instance-specific fields. Each excluded field has a comment
/// explaining why it's not part of the cache key.
///
/// # Arguments
/// * `params` - The post list parameters to generate a cache key from
///
/// # Returns
/// A URL query string containing only the filter-relevant parameters,
/// suitable for use as a cache key suffix.
///
/// # Example
/// ```ignore
/// let params = PostListParams {
///     status: vec![PostStatus::Publish],
///     author: vec![UserId(5)],
///     ..Default::default()
/// };
/// let key = post_list_params_cache_key(&params);
/// // key = "author=5&status=publish"
/// ```
pub fn post_list_params_cache_key(params: &PostListParams) -> String {
    let mut url = Url::parse("https://cache-key-generator.local").expect("valid base URL");

    {
        let mut q = url.query_pairs_mut();

        // ============================================================
        // EXCLUDED FIELDS (not part of cache key)
        // ============================================================

        // `page` - Excluded: pagination is managed by the collection, not the filter
        // `per_page` - Excluded: pagination is managed by the collection, not the filter
        // `offset` - Excluded: pagination is managed by the collection, not the filter
        // `include` - Excluded: instance-specific, used for fetching specific posts by ID
        // `exclude` - Excluded: instance-specific, used for excluding specific posts by ID

        // ============================================================
        // INCLUDED FIELDS (alphabetically ordered for determinism)
        // ============================================================

        // after - Filter: limit to posts published after this date
        q.append_option(PostListParamsField::After.into(), params.after.as_ref());

        // author - Filter: limit to posts by specific authors
        q.append_vec(PostListParamsField::Author.into(), &params.author);

        // author_exclude - Filter: exclude posts by specific authors
        q.append_vec(
            PostListParamsField::AuthorExclude.into(),
            &params.author_exclude,
        );

        // before - Filter: limit to posts published before this date
        q.append_option(PostListParamsField::Before.into(), params.before.as_ref());

        // categories - Filter: limit to posts in specific categories
        q.append_vec(PostListParamsField::Categories.into(), &params.categories);

        // categories_exclude - Filter: exclude posts in specific categories
        q.append_vec(
            PostListParamsField::CategoriesExclude.into(),
            &params.categories_exclude,
        );

        // menu_order - Filter: limit by menu order (for hierarchical post types)
        q.append_option(
            PostListParamsField::MenuOrder.into(),
            params.menu_order.as_ref(),
        );

        // modified_after - Filter: limit to posts modified after this date
        q.append_option(
            PostListParamsField::ModifiedAfter.into(),
            params.modified_after.as_ref(),
        );

        // modified_before - Filter: limit to posts modified before this date
        q.append_option(
            PostListParamsField::ModifiedBefore.into(),
            params.modified_before.as_ref(),
        );

        // order - Ordering: affects which posts appear on each page
        q.append_option(PostListParamsField::Order.into(), params.order.as_ref());

        // orderby - Ordering: affects which posts appear on each page
        q.append_option(PostListParamsField::Orderby.into(), params.orderby.as_ref());

        // parent - Filter: limit to posts with specific parent (hierarchical)
        q.append_option(PostListParamsField::Parent.into(), params.parent.as_ref());

        // parent_exclude - Filter: exclude posts with specific parents
        q.append_vec(
            PostListParamsField::ParentExclude.into(),
            &params.parent_exclude,
        );

        // search - Filter: limit to posts matching search string
        q.append_option(PostListParamsField::Search.into(), params.search.as_ref());

        // search_columns - Filter: which columns to search in
        q.append_vec(
            PostListParamsField::SearchColumns.into(),
            &params.search_columns,
        );

        // slug - Filter: limit to posts with specific slugs
        q.append_vec(PostListParamsField::Slug.into(), &params.slug);

        // status - Filter: limit to posts with specific statuses
        q.append_vec(PostListParamsField::Status.into(), &params.status);

        // sticky - Filter: limit to sticky or non-sticky posts
        q.append_option(PostListParamsField::Sticky.into(), params.sticky.as_ref());

        // tags - Filter: limit to posts with specific tags
        q.append_vec(PostListParamsField::Tags.into(), &params.tags);

        // tags_exclude - Filter: exclude posts with specific tags
        q.append_vec(
            PostListParamsField::TagsExclude.into(),
            &params.tags_exclude,
        );

        // tax_relation - Filter: relationship between taxonomy filters (AND/OR)
        q.append_option(
            PostListParamsField::TaxRelation.into(),
            params.tax_relation.as_ref(),
        );
    }

    url.query().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wp_api::posts::PostStatus;

    #[test]
    fn test_empty_params_produces_empty_key() {
        let params = PostListParams::default();
        let key = post_list_params_cache_key(&params);
        assert_eq!(key, "");
    }

    #[test]
    fn test_status_filter() {
        let params = PostListParams {
            status: vec![PostStatus::Publish],
            ..Default::default()
        };
        let key = post_list_params_cache_key(&params);
        assert_eq!(key, "status=publish");
    }

    #[test]
    fn test_multiple_statuses() {
        let params = PostListParams {
            status: vec![PostStatus::Publish, PostStatus::Draft],
            ..Default::default()
        };
        let key = post_list_params_cache_key(&params);
        assert_eq!(key, "status=publish%2Cdraft");
    }

    #[test]
    fn test_pagination_fields_excluded() {
        let params = PostListParams {
            page: Some(5),
            per_page: Some(20),
            offset: Some(100),
            status: vec![PostStatus::Publish],
            ..Default::default()
        };
        let key = post_list_params_cache_key(&params);
        // Should only contain status, not page/per_page/offset
        assert_eq!(key, "status=publish");
    }

    #[test]
    fn test_include_exclude_fields_excluded() {
        use wp_api::posts::PostId;

        let params = PostListParams {
            include: vec![PostId(1), PostId(2)],
            exclude: vec![PostId(3), PostId(4)],
            status: vec![PostStatus::Draft],
            ..Default::default()
        };
        let key = post_list_params_cache_key(&params);
        // Should only contain status, not include/exclude
        assert_eq!(key, "status=draft");
    }

    #[test]
    fn test_multiple_filters_alphabetically_ordered() {
        use wp_api::users::UserId;

        let params = PostListParams {
            status: vec![PostStatus::Publish],
            author: vec![UserId(5)],
            search: Some("hello".to_string()),
            ..Default::default()
        };
        let key = post_list_params_cache_key(&params);
        // Fields should be in alphabetical order: author, search, status
        assert_eq!(key, "author=5&search=hello&status=publish");
    }
}
