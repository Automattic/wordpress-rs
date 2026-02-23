//! Filter type for post metadata collections.
//!
//! This module provides `PostListFilter`, a subset of `PostListParams` containing
//! only fields appropriate for metadata collection filtering.

use std::cmp::Ordering;

use wp_api::{
    WpApiParamOrder,
    posts::{
        AnyPostWithEditContext, PostId, PostListParams, PostStatus, WpApiParamPostsOrderBy,
        WpApiParamPostsSearchColumn, WpApiParamPostsTaxRelation,
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
///     20, // per_page
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
    /// Check if a cached post loosely matches this filter.
    ///
    /// Conservative: returns `true` if the match cannot be determined locally.
    /// Only returns `false` when the post definitely does NOT match.
    ///
    /// Fields that require server-side data (search, categories, tags via term
    /// relationships) are not checked and assumed to match.
    pub fn loosely_matches_post(&self, post: &wp_api::posts::AnyPostWithEditContext) -> bool {
        // Status check
        if !self.status.is_empty() && !self.status.contains(&post.status) {
            return false;
        }
        // Author check
        if !self.author.is_empty()
            && let Some(author) = post.author
            && !self.author.contains(&author)
        {
            return false;
        }
        // Author exclude check
        if !self.author_exclude.is_empty()
            && let Some(author) = post.author
            && self.author_exclude.contains(&author)
        {
            return false;
        }
        // Sticky check
        if let Some(sticky_filter) = self.sticky
            && let Some(is_sticky) = post.sticky
            && is_sticky != sticky_filter
        {
            return false;
        }
        // Parent check
        if let Some(parent_filter) = self.parent
            && let Some(parent) = post.parent
            && parent != parent_filter
        {
            return false;
        }
        // Parent exclude check
        if !self.parent_exclude.is_empty()
            && let Some(parent) = post.parent
            && self.parent_exclude.contains(&parent)
        {
            return false;
        }
        // Slug check
        if !self.slug.is_empty() && !self.slug.contains(&post.slug) {
            return false;
        }
        true
    }

    /// Check if the filter's ordering is deterministic enough for local insert.
    ///
    /// An ordering is deterministic if we can compute the sort key from cached post data.
    /// Non-deterministic orderings (relevance, include, include_slugs) or unknown orderings
    /// (when orderby is None and the default depends on whether search is present)
    /// require a full list refresh instead.
    pub fn has_deterministic_ordering(&self) -> bool {
        match self.orderby {
            Some(WpApiParamPostsOrderBy::Date)
            | Some(WpApiParamPostsOrderBy::Modified)
            | Some(WpApiParamPostsOrderBy::Id)
            | Some(WpApiParamPostsOrderBy::Title)
            | Some(WpApiParamPostsOrderBy::MenuOrder)
            | Some(WpApiParamPostsOrderBy::Slug)
            | Some(WpApiParamPostsOrderBy::Author)
            | Some(WpApiParamPostsOrderBy::Parent) => true,
            Some(WpApiParamPostsOrderBy::Relevance)
            | Some(WpApiParamPostsOrderBy::Include)
            | Some(WpApiParamPostsOrderBy::IncludeSlugs) => false,
            None => {
                // WordPress default: date if no search, relevance if search is present
                self.search.is_none()
            }
        }
    }

    /// Get the effective order direction (default: Desc).
    pub fn effective_order(&self) -> WpApiParamOrder {
        self.order.unwrap_or(WpApiParamOrder::Desc)
    }

    /// Get the effective orderby field (default: Date).
    pub fn effective_orderby(&self) -> WpApiParamPostsOrderBy {
        self.orderby.unwrap_or(WpApiParamPostsOrderBy::Date)
    }

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

/// Compare two posts based on the given orderby and order direction.
///
/// Returns None if the sort key cannot be determined from cached data
/// (e.g., the required field is missing on one or both posts).
pub(crate) fn compare_posts_by_order(
    a: &AnyPostWithEditContext,
    b: &AnyPostWithEditContext,
    orderby: WpApiParamPostsOrderBy,
    order: WpApiParamOrder,
) -> Option<Ordering> {
    let cmp = match orderby {
        WpApiParamPostsOrderBy::Date => {
            // Use date_gmt for comparison (non-optional in edit context)
            a.date_gmt.0.cmp(&b.date_gmt.0)
        }
        WpApiParamPostsOrderBy::Modified => a.modified_gmt.0.cmp(&b.modified_gmt.0),
        WpApiParamPostsOrderBy::Id => a.id.0.cmp(&b.id.0),
        WpApiParamPostsOrderBy::Title => {
            let a_title = a
                .title
                .as_ref()
                .and_then(|t| t.raw.as_deref().or(Some(t.rendered.as_str())))?;
            let b_title = b
                .title
                .as_ref()
                .and_then(|t| t.raw.as_deref().or(Some(t.rendered.as_str())))?;
            a_title.cmp(b_title)
        }
        WpApiParamPostsOrderBy::Slug => a.slug.cmp(&b.slug),
        WpApiParamPostsOrderBy::MenuOrder => {
            let a_order = a.menu_order?;
            let b_order = b.menu_order?;
            a_order.cmp(&b_order)
        }
        WpApiParamPostsOrderBy::Author => {
            let a_author = a.author?;
            let b_author = b.author?;
            a_author.0.cmp(&b_author.0)
        }
        WpApiParamPostsOrderBy::Parent => {
            let a_parent = a.parent?;
            let b_parent = b.parent?;
            a_parent.0.cmp(&b_parent.0)
        }
        // Non-deterministic orderings should not reach here
        WpApiParamPostsOrderBy::Relevance
        | WpApiParamPostsOrderBy::Include
        | WpApiParamPostsOrderBy::IncludeSlugs => return None,
    };

    // Apply order direction
    Some(match order {
        WpApiParamOrder::Asc => cmp,
        WpApiParamOrder::Desc => cmp.reverse(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wp_mobile_cache::test_fixtures::posts::PostBuilder;

    // ============================================================
    // has_deterministic_ordering tests
    // ============================================================

    #[test]
    fn test_deterministic_ordering_date() {
        let filter = PostListFilter {
            orderby: Some(WpApiParamPostsOrderBy::Date),
            ..Default::default()
        };
        assert!(filter.has_deterministic_ordering());
    }

    #[test]
    fn test_deterministic_ordering_id() {
        let filter = PostListFilter {
            orderby: Some(WpApiParamPostsOrderBy::Id),
            ..Default::default()
        };
        assert!(filter.has_deterministic_ordering());
    }

    #[test]
    fn test_deterministic_ordering_modified() {
        let filter = PostListFilter {
            orderby: Some(WpApiParamPostsOrderBy::Modified),
            ..Default::default()
        };
        assert!(filter.has_deterministic_ordering());
    }

    #[test]
    fn test_deterministic_ordering_title() {
        let filter = PostListFilter {
            orderby: Some(WpApiParamPostsOrderBy::Title),
            ..Default::default()
        };
        assert!(filter.has_deterministic_ordering());
    }

    #[test]
    fn test_deterministic_ordering_menu_order() {
        let filter = PostListFilter {
            orderby: Some(WpApiParamPostsOrderBy::MenuOrder),
            ..Default::default()
        };
        assert!(filter.has_deterministic_ordering());
    }

    #[test]
    fn test_non_deterministic_ordering_relevance() {
        let filter = PostListFilter {
            orderby: Some(WpApiParamPostsOrderBy::Relevance),
            ..Default::default()
        };
        assert!(!filter.has_deterministic_ordering());
    }

    #[test]
    fn test_non_deterministic_ordering_include() {
        let filter = PostListFilter {
            orderby: Some(WpApiParamPostsOrderBy::Include),
            ..Default::default()
        };
        assert!(!filter.has_deterministic_ordering());
    }

    #[test]
    fn test_default_ordering_no_search() {
        // orderby=None, search=None => defaults to date => deterministic
        let filter = PostListFilter::default();
        assert!(filter.has_deterministic_ordering());
    }

    #[test]
    fn test_default_ordering_with_search() {
        // orderby=None, search=Some("query") => defaults to relevance => non-deterministic
        let filter = PostListFilter {
            search: Some("query".to_string()),
            ..Default::default()
        };
        assert!(!filter.has_deterministic_ordering());
    }

    // ============================================================
    // compare_posts_by_order tests
    // ============================================================

    #[test]
    fn test_compare_by_date_desc() {
        let post_a = PostBuilder::minimal().with_id(1).build();
        // post_b has a later date
        let mut post_b = PostBuilder::minimal().with_id(2).build();
        post_b.date_gmt = "2024-06-15T10:00:00Z".parse().unwrap();

        let result = compare_posts_by_order(
            &post_a,
            &post_b,
            WpApiParamPostsOrderBy::Date,
            WpApiParamOrder::Desc,
        );
        // Desc: later date should come first, so a (earlier) > b (later) in desc
        // a.date < b.date => cmp is Less, reversed => Greater
        assert_eq!(result, Some(Ordering::Greater));
    }

    #[test]
    fn test_compare_by_date_asc() {
        let post_a = PostBuilder::minimal().with_id(1).build();
        let mut post_b = PostBuilder::minimal().with_id(2).build();
        post_b.date_gmt = "2024-06-15T10:00:00Z".parse().unwrap();

        let result = compare_posts_by_order(
            &post_a,
            &post_b,
            WpApiParamPostsOrderBy::Date,
            WpApiParamOrder::Asc,
        );
        // Asc: earlier date first => a < b => Less
        assert_eq!(result, Some(Ordering::Less));
    }

    #[test]
    fn test_compare_by_id() {
        let post_a = PostBuilder::minimal().with_id(5).build();
        let post_b = PostBuilder::minimal().with_id(10).build();

        let result = compare_posts_by_order(
            &post_a,
            &post_b,
            WpApiParamPostsOrderBy::Id,
            WpApiParamOrder::Asc,
        );
        assert_eq!(result, Some(Ordering::Less));
    }

    #[test]
    fn test_compare_by_title() {
        let post_a = PostBuilder::minimal()
            .with_id(1)
            .with_title("Alpha")
            .build();
        let post_b = PostBuilder::minimal().with_id(2).with_title("Beta").build();

        let result = compare_posts_by_order(
            &post_a,
            &post_b,
            WpApiParamPostsOrderBy::Title,
            WpApiParamOrder::Asc,
        );
        assert_eq!(result, Some(Ordering::Less));
    }

    #[test]
    fn test_compare_missing_menu_order_returns_none() {
        // MenuOrder is optional; if missing on either post, compare returns None
        let post_a = PostBuilder::minimal().with_id(1).build();
        let post_b = PostBuilder::minimal().with_id(2).build();
        // Both posts have menu_order = None

        let result = compare_posts_by_order(
            &post_a,
            &post_b,
            WpApiParamPostsOrderBy::MenuOrder,
            WpApiParamOrder::Asc,
        );
        assert_eq!(result, None);
    }

    #[test]
    fn test_compare_relevance_returns_none() {
        let post_a = PostBuilder::minimal().with_id(1).build();
        let post_b = PostBuilder::minimal().with_id(2).build();

        let result = compare_posts_by_order(
            &post_a,
            &post_b,
            WpApiParamPostsOrderBy::Relevance,
            WpApiParamOrder::Desc,
        );
        assert_eq!(result, None);
    }

    // ============================================================
    // loosely_matches_post tests
    // ============================================================

    #[test]
    fn test_matches_empty_filter() {
        let filter = PostListFilter::default();
        let post = PostBuilder::minimal().build();
        assert!(filter.loosely_matches_post(&post));
    }

    #[test]
    fn test_matches_status() {
        let filter = PostListFilter {
            status: vec![PostStatus::Publish],
            ..Default::default()
        };
        let post = PostBuilder::minimal()
            .with_status(PostStatus::Publish)
            .build();
        assert!(filter.loosely_matches_post(&post));
    }

    #[test]
    fn test_no_match_wrong_status() {
        let filter = PostListFilter {
            status: vec![PostStatus::Publish],
            ..Default::default()
        };
        let post = PostBuilder::minimal()
            .with_status(PostStatus::Draft)
            .build();
        assert!(!filter.loosely_matches_post(&post));
    }

    #[test]
    fn test_matches_when_category_filter_present() {
        // Category filters cannot be checked locally, so the filter is conservative
        // and returns true even when categories are specified.
        let filter = PostListFilter {
            categories: vec![TermId(5), TermId(10)],
            ..Default::default()
        };
        let post = PostBuilder::minimal().build();
        assert!(filter.loosely_matches_post(&post));
    }
}
