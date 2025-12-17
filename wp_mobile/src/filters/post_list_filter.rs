//! Filter type for post metadata collections.
//!
//! This module provides `PostListFilter`, a subset of `PostListParams` containing
//! only fields appropriate for metadata collection filtering.

use wp_api::{
    WpApiParamOrder,
    posts::{
        PostId, PostListParams, PostStatus, WpApiParamPostsOrderBy, WpApiParamPostsSearchColumn,
        WpApiParamPostsTaxRelation,
    },
    terms::TermId,
    users::UserId,
};

/// Filter parameters for post metadata collections.
///
/// This type exposes only the filter-relevant fields from `PostListParams`,
/// excluding fields that are inappropriate for metadata collection use cases.
///
/// # Excluded Fields
///
/// The following `PostListParams` fields are intentionally excluded:
///
/// ## Pagination fields (managed by the collection)
/// - `page` - The collection manages pagination internally via `refresh()` and `load_next_page()`
/// - `per_page` - The collection uses a fixed page size for consistent syncing
/// - `offset` - Incompatible with the collection's page-based pagination model
///
/// ## Instance-specific fields (not suitable for cached lists)
/// - `include` - For fetching specific posts by ID; use direct entity fetching instead
/// - `exclude` - For excluding specific posts; would require cache invalidation on every change
///
/// ## Date range fields (incompatible with metadata sync model)
/// - `after` - Date-bounded queries don't fit the "sync all matching posts" model
/// - `modified_after` - Same reason; the collection tracks modifications via `modified_gmt`
/// - `before` - Date-bounded queries create incomplete views that can't be reliably synced
/// - `modified_before` - Same reason; would miss posts modified after the boundary
///
/// # Usage
///
/// ```ignore
/// let filter = PostListFilter {
///     status: vec![PostStatus::Publish],
///     orderby: Some(WpApiParamPostsOrderBy::Date),
///     order: Some(WpApiParamOrder::Desc),
///     ..Default::default()
/// };
///
/// let collection = post_service.create_post_metadata_collection_with_edit_context(
///     PostEndpointType::Posts,
///     filter,
/// );
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PostListFilter {
    // ============================================================
    // Text Search
    // ============================================================
    /// Limit results to those matching a string.
    #[uniffi(default = None)]
    pub search: Option<String>,

    /// Array of column names to be searched.
    #[uniffi(default = [])]
    pub search_columns: Vec<WpApiParamPostsSearchColumn>,

    // ============================================================
    // Author Filtering
    // ============================================================
    /// Limit result set to posts assigned to specific authors.
    #[uniffi(default = [])]
    pub author: Vec<UserId>,

    /// Ensure result set excludes posts assigned to specific authors.
    #[uniffi(default = [])]
    pub author_exclude: Vec<UserId>,

    // ============================================================
    // Ordering
    // ============================================================
    /// Order sort attribute ascending or descending.
    /// Default: desc
    #[uniffi(default = None)]
    pub order: Option<WpApiParamOrder>,

    /// Sort collection by post attribute.
    /// Default: date
    #[uniffi(default = None)]
    pub orderby: Option<WpApiParamPostsOrderBy>,

    // ============================================================
    // Slug Filtering
    // ============================================================
    /// Limit result set to posts with one or more specific slugs.
    #[uniffi(default = [])]
    pub slug: Vec<String>,

    // ============================================================
    // Status Filtering
    // ============================================================
    /// Limit result set to posts assigned one or more statuses.
    /// Default: publish
    #[uniffi(default = [])]
    pub status: Vec<PostStatus>,

    // ============================================================
    // Taxonomy Filtering
    // ============================================================
    /// Limit result set based on relationship between multiple taxonomies.
    /// One of: AND, OR
    #[uniffi(default = None)]
    pub tax_relation: Option<WpApiParamPostsTaxRelation>,

    /// Limit result set to items with specific terms assigned in the categories taxonomy.
    #[uniffi(default = [])]
    pub categories: Vec<TermId>,

    /// Limit result set to items except those with specific terms assigned in the categories taxonomy.
    #[uniffi(default = [])]
    pub categories_exclude: Vec<TermId>,

    /// Limit result set to items with specific terms assigned in the tags taxonomy.
    #[uniffi(default = [])]
    pub tags: Vec<TermId>,

    /// Limit result set to items except those with specific terms assigned in the tags taxonomy.
    #[uniffi(default = [])]
    pub tags_exclude: Vec<TermId>,

    // ============================================================
    // Sticky Posts
    // ============================================================
    /// Limit result set to items that are sticky.
    #[uniffi(default = None)]
    pub sticky: Option<bool>,

    // ============================================================
    // Hierarchical Post Type Fields (pages, etc.)
    // ============================================================
    /// Limit result set to items with a specific parent.
    #[uniffi(default = None)]
    pub parent: Option<PostId>,

    /// Limit result set to items except those of a specific parent.
    #[uniffi(default = [])]
    pub parent_exclude: Vec<PostId>,

    /// Limit result set by menu order.
    #[uniffi(default = None)]
    pub menu_order: Option<u32>,
}

impl PostListFilter {
    /// Convert filter to `PostListParams` for API requests.
    ///
    /// This creates a `PostListParams` with pagination fields set by the caller
    /// (typically the service layer) and filter fields from this struct.
    ///
    /// # Arguments
    /// * `page` - Page number for the request
    /// * `per_page` - Number of items per page
    pub fn to_list_params(&self, page: u32, per_page: u32) -> PostListParams {
        PostListParams {
            // Pagination (provided by caller)
            page: Some(page),
            per_page: Some(per_page),

            // Filter fields (from self)
            search: self.search.clone(),
            search_columns: self.search_columns.clone(),
            author: self.author.clone(),
            author_exclude: self.author_exclude.clone(),
            order: self.order,
            orderby: self.orderby,
            slug: self.slug.clone(),
            status: self.status.clone(),
            tax_relation: self.tax_relation,
            categories: self.categories.clone(),
            categories_exclude: self.categories_exclude.clone(),
            tags: self.tags.clone(),
            tags_exclude: self.tags_exclude.clone(),
            sticky: self.sticky,
            parent: self.parent,
            parent_exclude: self.parent_exclude.clone(),
            menu_order: self.menu_order,

            // Excluded fields (set to defaults)
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
