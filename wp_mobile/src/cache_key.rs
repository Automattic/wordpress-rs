//! Cache key generation for metadata collections.
//!
//! This module provides functions to generate deterministic cache keys from
//! filter parameters. The cache key is used to identify unique list configurations
//! in the metadata store.

use url::Url;
use wp_api::{
    media::MediaListParamsField, posts::PostListParamsField,
    request::endpoint::posts_endpoint::PostEndpointType, url_query::QueryPairsExtension,
};

use crate::filters::{MediaListFilter, PostListFilter};

/// Generates a cache key segment from a `PostEndpointType`.
///
/// Uses a `post_type_` prefix to avoid conflicts with custom post type names
/// that might match other cache key segments.
///
/// # Returns
/// A string suitable for use in cache keys:
/// - `PostEndpointType::Posts` → `"post_type_posts"`
/// - `PostEndpointType::Pages` → `"post_type_pages"`
/// - `PostEndpointType::Custom(name)` → `"post_type_custom_{name}"`
///
/// # Example
/// ```ignore
/// let key = endpoint_type_cache_key(&PostEndpointType::Posts);
/// assert_eq!(key, "post_type_posts");
///
/// let key = endpoint_type_cache_key(&PostEndpointType::Custom("products".to_string()));
/// assert_eq!(key, "post_type_custom_products");
/// ```
pub fn endpoint_type_cache_key(endpoint_type: &PostEndpointType) -> String {
    match endpoint_type {
        PostEndpointType::Posts => "post_type_posts".to_string(),
        PostEndpointType::Pages => "post_type_pages".to_string(),
        PostEndpointType::Custom(name) => format!("post_type_custom_{}", name),
    }
}

/// Generates a deterministic cache key from `PostListFilter`.
///
/// All fields in `PostListFilter` are included in the cache key since it only
/// contains filter-relevant fields (pagination, instance-specific, and date
/// range fields are excluded by design in `PostListFilter`).
///
/// # Arguments
/// * `filter` - The post list filter to generate a cache key from
///
/// # Returns
/// A URL query string containing the filter parameters in alphabetical order,
/// suitable for use as a cache key suffix.
///
/// # Example
/// ```ignore
/// let filter = PostListFilter {
///     status: vec![PostStatus::Publish],
///     author: vec![UserId(5)],
///     ..Default::default()
/// };
/// let key = post_list_filter_cache_key(&filter);
/// // key = "author=5&status=publish"
/// ```
pub fn post_list_filter_cache_key(filter: &PostListFilter) -> String {
    let mut url = Url::parse("https://cache-key-generator.local").expect("valid base URL");

    {
        let mut q = url.query_pairs_mut();

        // All fields in PostListFilter are included (alphabetically ordered for determinism).
        // Fields excluded from PostListFilter (pagination, instance-specific, date ranges)
        // are documented in the PostListFilter type definition.

        q.append_vec_query_value_pair(PostListParamsField::Author, &filter.author);
        q.append_vec_query_value_pair(PostListParamsField::AuthorExclude, &filter.author_exclude);
        q.append_vec_query_value_pair(PostListParamsField::Categories, &filter.categories);
        q.append_vec_query_value_pair(
            PostListParamsField::CategoriesExclude,
            &filter.categories_exclude,
        );
        q.append_option_query_value_pair(
            PostListParamsField::MenuOrder,
            filter.menu_order.as_ref(),
        );
        q.append_option_query_value_pair(PostListParamsField::Order, filter.order.as_ref());
        q.append_option_query_value_pair(PostListParamsField::Orderby, filter.orderby.as_ref());
        q.append_option_query_value_pair(PostListParamsField::Parent, filter.parent.as_ref());
        q.append_vec_query_value_pair(PostListParamsField::ParentExclude, &filter.parent_exclude);
        q.append_option_query_value_pair(PostListParamsField::Search, filter.search.as_ref());
        q.append_vec_query_value_pair(PostListParamsField::SearchColumns, &filter.search_columns);
        q.append_vec_query_value_pair(PostListParamsField::Slug, &filter.slug);
        q.append_vec_query_value_pair(PostListParamsField::Status, &filter.status);
        q.append_option_query_value_pair(PostListParamsField::Sticky, filter.sticky.as_ref());
        q.append_vec_query_value_pair(PostListParamsField::Tags, &filter.tags);
        q.append_vec_query_value_pair(PostListParamsField::TagsExclude, &filter.tags_exclude);
        q.append_option_query_value_pair(
            PostListParamsField::TaxRelation,
            filter.tax_relation.as_ref(),
        );
    }

    url.query().unwrap_or("").to_string()
}

/// Generates a deterministic cache key from `MediaListFilter`.
///
/// All fields in `MediaListFilter` are included in the cache key since it only
/// contains filter-relevant fields (pagination, instance-specific, and date
/// range fields are excluded by design in `MediaListFilter`).
pub fn media_list_filter_cache_key(filter: &MediaListFilter) -> String {
    let mut url = Url::parse("https://cache-key-generator.local").expect("valid base URL");

    {
        let mut q = url.query_pairs_mut();

        // Alphabetically ordered for determinism.
        q.append_vec_query_value_pair(MediaListParamsField::Author, &filter.author);
        q.append_vec_query_value_pair(MediaListParamsField::AuthorExclude, &filter.author_exclude);
        q.append_option_query_value_pair(
            MediaListParamsField::MediaType,
            filter.media_type.as_ref(),
        );
        q.append_option_query_value_pair(MediaListParamsField::MimeType, filter.mime_type.as_ref());
        q.append_option_query_value_pair(MediaListParamsField::Order, filter.order.as_ref());
        q.append_option_query_value_pair(MediaListParamsField::Orderby, filter.orderby.as_ref());
        q.append_vec_query_value_pair(MediaListParamsField::Parent, &filter.parent);
        q.append_vec_query_value_pair(MediaListParamsField::ParentExclude, &filter.parent_exclude);
        q.append_option_query_value_pair(MediaListParamsField::Search, filter.search.as_ref());
        q.append_vec_query_value_pair(MediaListParamsField::SearchColumns, &filter.search_columns);
        q.append_vec_query_value_pair(MediaListParamsField::Slug, &filter.slug);
        q.append_vec_query_value_pair(MediaListParamsField::Status, &filter.status);
    }

    url.query().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wp_api::posts::PostStatus;

    #[test]
    fn test_empty_filter_produces_empty_key() {
        let filter = PostListFilter::default();
        let key = post_list_filter_cache_key(&filter);
        assert_eq!(key, "");
    }

    #[test]
    fn test_status_filter() {
        let filter = PostListFilter {
            status: vec![PostStatus::Publish],
            ..Default::default()
        };
        let key = post_list_filter_cache_key(&filter);
        assert_eq!(key, "status=publish");
    }

    #[test]
    fn test_multiple_statuses() {
        let filter = PostListFilter {
            status: vec![PostStatus::Publish, PostStatus::Draft],
            ..Default::default()
        };
        let key = post_list_filter_cache_key(&filter);
        assert_eq!(key, "status=publish%2Cdraft");
    }

    #[test]
    fn test_multiple_filters_alphabetically_ordered() {
        use wp_api::users::UserId;

        let filter = PostListFilter {
            status: vec![PostStatus::Publish],
            author: vec![UserId(5)],
            search: Some("hello".to_string()),
            ..Default::default()
        };
        let key = post_list_filter_cache_key(&filter);
        // Fields should be in alphabetical order: author, search, status
        assert_eq!(key, "author=5&search=hello&status=publish");
    }

    #[test]
    fn test_endpoint_type_posts() {
        let key = endpoint_type_cache_key(&PostEndpointType::Posts);
        assert_eq!(key, "post_type_posts");
    }

    #[test]
    fn test_endpoint_type_pages() {
        let key = endpoint_type_cache_key(&PostEndpointType::Pages);
        assert_eq!(key, "post_type_pages");
    }

    #[test]
    fn test_endpoint_type_custom() {
        let key = endpoint_type_cache_key(&PostEndpointType::Custom("products".to_string()));
        assert_eq!(key, "post_type_custom_products");
    }

    #[test]
    fn media_empty_filter_produces_empty_key() {
        let filter = MediaListFilter::default();
        let key = media_list_filter_cache_key(&filter);
        assert_eq!(key, "");
    }

    #[test]
    fn media_status_filter() {
        use wp_api::media::MediaStatus;
        let filter = MediaListFilter {
            status: vec![MediaStatus::Inherit],
            ..Default::default()
        };
        let key = media_list_filter_cache_key(&filter);
        assert_eq!(key, "status=inherit");
    }

    #[test]
    fn media_multi_field_sorted() {
        use wp_api::media::{MediaStatus, MediaTypeParam};
        use wp_api::users::UserId;
        let filter = MediaListFilter {
            status: vec![MediaStatus::Inherit],
            author: vec![UserId(5)],
            media_type: Some(MediaTypeParam::Image),
            ..Default::default()
        };
        let key = media_list_filter_cache_key(&filter);
        // Fields in alphabetical order: author, media_type, status
        assert_eq!(key, "author=5&media_type=image&status=inherit");
    }
}
