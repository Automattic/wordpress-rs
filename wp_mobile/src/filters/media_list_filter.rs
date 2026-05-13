//! Filter type for media metadata collections.
//!
//! This module provides `MediaListFilter`, a subset of `MediaListParams` containing
//! only fields appropriate for metadata collection filtering.

use wp_api::{
    WpApiParamOrder,
    media::{MediaListParams, MediaStatus, MediaTypeParam},
    posts::{PostId, WpApiParamPostsOrderBy, WpApiParamPostsSearchColumn},
    users::UserId,
};

/// Filter parameters for media metadata collections.
///
/// This type exposes only the filter-relevant fields from `MediaListParams`,
/// excluding pagination, instance-specific lookup (`include`/`exclude`), and
/// date-range fields. Those are inappropriate for metadata-collection use
/// cases for the same reasons documented on `PostListFilter`.
#[derive(Debug, Default, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MediaListFilter {
    /// Limit results to those matching a string.
    #[uniffi(default = None)]
    pub search: Option<String>,

    /// Array of column names to be searched.
    #[uniffi(default = [])]
    pub search_columns: Vec<WpApiParamPostsSearchColumn>,

    /// Limit result set to media assigned to specific authors.
    #[uniffi(default = [])]
    pub author: Vec<UserId>,

    /// Ensure result set excludes media assigned to specific authors.
    #[uniffi(default = [])]
    pub author_exclude: Vec<UserId>,

    /// Order sort attribute ascending or descending.
    /// Default: desc
    #[uniffi(default = None)]
    pub order: Option<WpApiParamOrder>,

    /// Sort collection by media attribute.
    /// Default: date
    #[uniffi(default = None)]
    pub orderby: Option<WpApiParamPostsOrderBy>,

    /// Limit result set to media with one or more specific slugs.
    #[uniffi(default = [])]
    pub slug: Vec<String>,

    /// Limit result set to media assigned one or more statuses.
    /// Default: inherit
    #[uniffi(default = [])]
    pub status: Vec<MediaStatus>,

    /// Limit result set to media attached to one of these posts.
    #[uniffi(default = [])]
    pub parent: Vec<PostId>,

    /// Exclude media attached to any of these posts.
    #[uniffi(default = [])]
    pub parent_exclude: Vec<PostId>,

    /// Limit result set to attachments of a particular media type.
    #[uniffi(default = None)]
    pub media_type: Option<MediaTypeParam>,

    /// Limit result set to attachments of a particular MIME type.
    #[uniffi(default = None)]
    pub mime_type: Option<String>,
}

impl MediaListFilter {
    /// Convert this filter into a `MediaListParams` ready for an API call.
    ///
    /// Pagination is provided by the caller (the service layer) since
    /// `MediaListFilter` deliberately omits page/per_page/offset.
    pub fn to_list_params(&self, page: u32, per_page: u32) -> MediaListParams {
        MediaListParams {
            page: Some(page),
            per_page: Some(per_page),
            search: self.search.clone(),
            search_columns: self.search_columns.clone(),
            author: self.author.clone(),
            author_exclude: self.author_exclude.clone(),
            order: self.order,
            orderby: self.orderby,
            slug: self.slug.clone(),
            status: self.status.clone(),
            parent: self.parent.clone(),
            parent_exclude: self.parent_exclude.clone(),
            media_type: self.media_type.clone(),
            mime_type: self.mime_type.clone(),
            // Fields intentionally excluded from MediaListFilter.
            offset: None,
            include: Vec::new(),
            exclude: Vec::new(),
            after: None,
            modified_after: None,
            before: None,
            modified_before: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_list_params_sets_page_and_per_page() {
        let filter = MediaListFilter::default();
        let params = filter.to_list_params(3, 25);
        assert_eq!(params.page, Some(3));
        assert_eq!(params.per_page, Some(25));
    }

    #[test]
    fn to_list_params_clones_search_columns_and_status() {
        let filter = MediaListFilter {
            search_columns: vec![WpApiParamPostsSearchColumn::PostTitle],
            status: vec![MediaStatus::Inherit, MediaStatus::Private],
            ..Default::default()
        };
        let params = filter.to_list_params(1, 10);
        assert_eq!(
            params.search_columns,
            vec![WpApiParamPostsSearchColumn::PostTitle]
        );
        assert_eq!(
            params.status,
            vec![MediaStatus::Inherit, MediaStatus::Private]
        );
    }

    #[test]
    fn to_list_params_passes_media_type_and_mime_type_through() {
        let filter = MediaListFilter {
            media_type: Some(MediaTypeParam::Image),
            mime_type: Some("image/jpeg".to_string()),
            ..Default::default()
        };
        let params = filter.to_list_params(1, 10);
        assert_eq!(params.media_type, Some(MediaTypeParam::Image));
        assert_eq!(params.mime_type, Some("image/jpeg".to_string()));
    }
}
