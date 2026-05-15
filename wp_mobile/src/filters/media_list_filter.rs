//! Filter type for media metadata collections.
//!
//! This module provides `MediaListFilter`, a subset of `MediaListParams` containing
//! only fields appropriate for metadata collection filtering.

use std::cmp::Ordering;

use wp_api::{
    WpApiParamOrder,
    media::{MediaListParams, MediaStatus, MediaType, MediaTypeParam, MediaWithEditContext},
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

impl MediaListFilter {
    /// Check if a cached media item loosely matches this filter, mimicking the
    /// WordPress REST API's query parameter behavior.
    ///
    /// Conservative: returns `true` if the match cannot be determined locally.
    /// Only returns `false` when the media definitely does NOT match.
    ///
    /// Field-by-field:
    /// - `status`, `author`, `author_exclude`, `slug`, `parent`, `parent_exclude`,
    ///   `media_type`, `mime_type` — checked locally (definite no → false).
    /// - `search`, `search_columns` — server-only; assumed to match.
    pub fn loosely_matches_media(&self, media: &MediaWithEditContext) -> bool {
        // Status check
        if !self.status.is_empty() && !self.status.contains(&media.status) {
            return false;
        }
        // Author check
        if !self.author.is_empty() && !self.author.contains(&media.author) {
            return false;
        }
        // Author exclude check
        if !self.author_exclude.is_empty() && self.author_exclude.contains(&media.author) {
            return false;
        }
        // Slug check
        if !self.slug.is_empty() && !self.slug.contains(&media.slug) {
            return false;
        }
        // Parent check (media's attached post id). A non-empty `parent` filter
        // rejects media with no post_id — they cannot belong to any post.
        if !self.parent.is_empty() {
            let Some(post_id) = media.post_id else {
                return false;
            };
            if !self.parent.contains(&post_id) {
                return false;
            }
        }
        // Parent exclude check
        if !self.parent_exclude.is_empty()
            && let Some(post_id) = media.post_id
            && self.parent_exclude.contains(&post_id)
        {
            return false;
        }
        // Media type check.
        if let Some(param) = &self.media_type {
            let media_mime_top = media.mime_type.split_once('/').map(|(top, _)| top);
            let matches = match (param, &media.media_type) {
                (MediaTypeParam::Image, MediaType::Image) => true,
                (MediaTypeParam::Image, MediaType::File) => false,
                (MediaTypeParam::Video, MediaType::File) => media_mime_top == Some("video"),
                (MediaTypeParam::Audio, MediaType::File) => media_mime_top == Some("audio"),
                (MediaTypeParam::Application, MediaType::File) => {
                    media_mime_top == Some("application")
                }
                (MediaTypeParam::Text, MediaType::File) => media_mime_top == Some("text"),
                (
                    MediaTypeParam::Video
                    | MediaTypeParam::Audio
                    | MediaTypeParam::Application
                    | MediaTypeParam::Text,
                    MediaType::Image,
                ) => false,
                (MediaTypeParam::Custom(_), _) | (_, MediaType::Custom(_)) => true,
            };
            if !matches {
                return false;
            }
        }
        // Mime type check.
        if let Some(filter_mime) = &self.mime_type {
            let media_mime = &media.mime_type;
            let matches = if filter_mime.contains('/') {
                filter_mime == media_mime
            } else {
                media_mime
                    .split_once('/')
                    .map(|(top, _)| top == filter_mime)
                    .unwrap_or(false)
            };
            if !matches {
                return false;
            }
        }
        true
    }

    pub fn has_deterministic_ordering(&self) -> bool {
        match self.orderby {
            Some(WpApiParamPostsOrderBy::Date)
            | Some(WpApiParamPostsOrderBy::Modified)
            | Some(WpApiParamPostsOrderBy::Id)
            | Some(WpApiParamPostsOrderBy::Title)
            | Some(WpApiParamPostsOrderBy::Slug)
            | Some(WpApiParamPostsOrderBy::Author)
            | Some(WpApiParamPostsOrderBy::Parent) => true,
            Some(WpApiParamPostsOrderBy::Relevance)
            | Some(WpApiParamPostsOrderBy::Include)
            | Some(WpApiParamPostsOrderBy::IncludeSlugs)
            | Some(WpApiParamPostsOrderBy::MenuOrder) => false,
            None => self.search.is_none(),
        }
    }

    pub fn effective_order(&self) -> WpApiParamOrder {
        self.order.unwrap_or(WpApiParamOrder::Desc)
    }

    pub fn effective_orderby(&self) -> WpApiParamPostsOrderBy {
        self.orderby.unwrap_or(WpApiParamPostsOrderBy::Date)
    }
}

pub(crate) fn compare_media_by_order(
    a: &MediaWithEditContext,
    b: &MediaWithEditContext,
    orderby: WpApiParamPostsOrderBy,
    order: WpApiParamOrder,
) -> Option<Ordering> {
    let cmp = match orderby {
        WpApiParamPostsOrderBy::Date => a.date_gmt.0.cmp(&b.date_gmt.0),
        WpApiParamPostsOrderBy::Modified => a.modified_gmt.0.cmp(&b.modified_gmt.0),
        WpApiParamPostsOrderBy::Id => a.id.0.cmp(&b.id.0),
        WpApiParamPostsOrderBy::Title => {
            let a_title = a.title.raw.as_deref().unwrap_or(a.title.rendered.as_str());
            let b_title = b.title.raw.as_deref().unwrap_or(b.title.rendered.as_str());
            a_title.cmp(b_title)
        }
        WpApiParamPostsOrderBy::Slug => a.slug.cmp(&b.slug),
        WpApiParamPostsOrderBy::Author => a.author.0.cmp(&b.author.0),
        WpApiParamPostsOrderBy::Parent => {
            let a_parent = a.post_id?.0;
            let b_parent = b.post_id?.0;
            a_parent.cmp(&b_parent)
        }
        WpApiParamPostsOrderBy::Relevance
        | WpApiParamPostsOrderBy::Include
        | WpApiParamPostsOrderBy::IncludeSlugs
        | WpApiParamPostsOrderBy::MenuOrder => return None,
    };

    Some(match order {
        WpApiParamOrder::Asc => cmp,
        WpApiParamOrder::Desc => cmp.reverse(),
    })
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

    use wp_api::media::MediaType;
    use wp_mobile_cache::test_fixtures::media::MediaBuilder;

    #[test]
    fn loosely_matches_media_returns_true_for_empty_filter() {
        let filter = MediaListFilter::default();
        let media = MediaBuilder::minimal().build();
        assert!(filter.loosely_matches_media(&media));
    }

    #[test]
    fn loosely_matches_media_rejects_status_mismatch() {
        let filter = MediaListFilter {
            status: vec![MediaStatus::Inherit],
            ..Default::default()
        };
        let media = MediaBuilder::minimal()
            .with_status(MediaStatus::Private)
            .build();
        assert!(!filter.loosely_matches_media(&media));
    }

    #[test]
    fn loosely_matches_media_accepts_status_match() {
        let filter = MediaListFilter {
            status: vec![MediaStatus::Inherit, MediaStatus::Private],
            ..Default::default()
        };
        let media = MediaBuilder::minimal()
            .with_status(MediaStatus::Private)
            .build();
        assert!(filter.loosely_matches_media(&media));
    }

    #[test]
    fn loosely_matches_media_rejects_excluded_author() {
        let filter = MediaListFilter {
            author_exclude: vec![UserId(7)],
            ..Default::default()
        };
        let media = MediaBuilder::minimal().with_author(UserId(7)).build();
        assert!(!filter.loosely_matches_media(&media));
    }

    #[test]
    fn loosely_matches_media_rejects_parent_mismatch_when_filter_set() {
        let filter = MediaListFilter {
            parent: vec![PostId(42)],
            ..Default::default()
        };
        let media = MediaBuilder::minimal().with_post_id(PostId(99)).build();
        assert!(!filter.loosely_matches_media(&media));
    }

    #[test]
    fn loosely_matches_media_rejects_unattached_media_when_parent_filter_set() {
        // Parent filter set but media has no post_id (None). Must reject —
        // the let-chain bug from review 1 would incorrectly accept this.
        let filter = MediaListFilter {
            parent: vec![PostId(42)],
            ..Default::default()
        };
        let media = MediaBuilder::minimal().build(); // post_id defaults to None
        assert!(!filter.loosely_matches_media(&media));
    }

    #[test]
    fn loosely_matches_media_accepts_unattached_media_when_parent_exclude_set() {
        // Symmetric to rejects_unattached_media_when_parent_filter_set:
        // a non-empty `parent_exclude` filter with `media.post_id == None`
        // conservatively passes — unattached media can't be in the exclude list.
        let filter = MediaListFilter {
            parent_exclude: vec![PostId(42)],
            ..Default::default()
        };
        let media = MediaBuilder::minimal().build(); // post_id defaults to None
        assert!(filter.loosely_matches_media(&media));
    }

    #[test]
    fn loosely_matches_media_rejects_slug_mismatch() {
        let filter = MediaListFilter {
            slug: vec!["wanted".to_string()],
            ..Default::default()
        };
        let media = MediaBuilder::minimal().with_slug("unwanted").build();
        assert!(!filter.loosely_matches_media(&media));
    }

    #[test]
    fn loosely_matches_media_rejects_media_type_mismatch_image_only() {
        // Image-only filter must reject MediaType::File (videos, audio,
        // documents all bucket to File on the actual media item — see the
        // MediaType vs MediaTypeParam comment in wp_api/src/media.rs).
        let filter = MediaListFilter {
            media_type: Some(MediaTypeParam::Image),
            ..Default::default()
        };
        let media = MediaBuilder::minimal()
            .with_media_type(MediaType::File)
            .build();
        assert!(!filter.loosely_matches_media(&media));
    }

    #[test]
    fn loosely_matches_media_rejects_image_filter_for_file_bucketing() {
        // Video/Audio/Application filter must reject MediaType::Image.
        for param in [
            MediaTypeParam::Video,
            MediaTypeParam::Audio,
            MediaTypeParam::Application,
            MediaTypeParam::Text,
        ] {
            let filter = MediaListFilter {
                media_type: Some(param.clone()),
                ..Default::default()
            };
            let media = MediaBuilder::minimal()
                .with_media_type(MediaType::Image)
                .build();
            assert!(
                !filter.loosely_matches_media(&media),
                "{:?} filter should reject Image-typed media",
                param
            );
        }
    }

    #[test]
    fn loosely_matches_media_accepts_video_filter_for_video_mime_file() {
        let filter = MediaListFilter {
            media_type: Some(MediaTypeParam::Video),
            ..Default::default()
        };
        let media = MediaBuilder::minimal()
            .with_media_type(MediaType::File)
            .with_mime_type("video/mp4")
            .build();
        assert!(filter.loosely_matches_media(&media));
    }

    #[test]
    fn loosely_matches_media_rejects_video_filter_for_pdf() {
        let filter = MediaListFilter {
            media_type: Some(MediaTypeParam::Video),
            ..Default::default()
        };
        let media = MediaBuilder::minimal()
            .with_media_type(MediaType::File)
            .with_mime_type("application/pdf")
            .build();
        assert!(!filter.loosely_matches_media(&media));
    }

    #[test]
    fn loosely_matches_media_accepts_application_filter_for_pdf() {
        let filter = MediaListFilter {
            media_type: Some(MediaTypeParam::Application),
            ..Default::default()
        };
        let media = MediaBuilder::minimal()
            .with_media_type(MediaType::File)
            .with_mime_type("application/pdf")
            .build();
        assert!(filter.loosely_matches_media(&media));
    }

    #[test]
    fn loosely_matches_media_rejects_audio_filter_for_video_mime() {
        let filter = MediaListFilter {
            media_type: Some(MediaTypeParam::Audio),
            ..Default::default()
        };
        let media = MediaBuilder::minimal()
            .with_media_type(MediaType::File)
            .with_mime_type("video/mp4")
            .build();
        assert!(!filter.loosely_matches_media(&media));
    }

    #[test]
    fn loosely_matches_media_rejects_mime_type_exact_mismatch() {
        let filter = MediaListFilter {
            mime_type: Some("image/png".to_string()),
            ..Default::default()
        };
        let media = MediaBuilder::minimal().with_mime_type("image/jpeg").build();
        assert!(!filter.loosely_matches_media(&media));
    }

    #[test]
    fn loosely_matches_media_accepts_mime_type_exact_match() {
        let filter = MediaListFilter {
            mime_type: Some("image/jpeg".to_string()),
            ..Default::default()
        };
        let media = MediaBuilder::minimal().with_mime_type("image/jpeg").build();
        assert!(filter.loosely_matches_media(&media));
    }

    #[test]
    fn loosely_matches_media_accepts_mime_type_prefix_filter() {
        let filter = MediaListFilter {
            mime_type: Some("image".to_string()),
            ..Default::default()
        };
        let media = MediaBuilder::minimal().with_mime_type("image/png").build();
        assert!(filter.loosely_matches_media(&media));
    }

    #[test]
    fn loosely_matches_media_rejects_mime_type_prefix_mismatch() {
        let filter = MediaListFilter {
            mime_type: Some("video".to_string()),
            ..Default::default()
        };
        let media = MediaBuilder::minimal().with_mime_type("image/png").build();
        assert!(!filter.loosely_matches_media(&media));
    }

    #[test]
    fn deterministic_ordering_returns_true_for_date() {
        let filter = MediaListFilter {
            orderby: Some(WpApiParamPostsOrderBy::Date),
            ..Default::default()
        };
        assert!(filter.has_deterministic_ordering());
    }
    #[test]
    fn deterministic_ordering_returns_false_for_relevance() {
        let filter = MediaListFilter {
            orderby: Some(WpApiParamPostsOrderBy::Relevance),
            ..Default::default()
        };
        assert!(!filter.has_deterministic_ordering());
    }
    #[test]
    fn deterministic_ordering_default_with_search_is_false() {
        let filter = MediaListFilter {
            search: Some("hello".to_string()),
            ..Default::default()
        };
        assert!(!filter.has_deterministic_ordering());
    }
    #[test]
    fn deterministic_ordering_default_without_search_is_true() {
        let filter = MediaListFilter::default();
        assert!(filter.has_deterministic_ordering());
    }
    #[test]
    fn effective_order_defaults_to_desc() {
        let filter = MediaListFilter::default();
        assert_eq!(filter.effective_order(), WpApiParamOrder::Desc);
    }
    #[test]
    fn effective_orderby_defaults_to_date() {
        let filter = MediaListFilter::default();
        assert_eq!(filter.effective_orderby(), WpApiParamPostsOrderBy::Date);
    }

    #[test]
    fn compare_media_by_id_asc() {
        let a = MediaBuilder::minimal().with_id(1).build();
        let b = MediaBuilder::minimal().with_id(2).build();
        let result =
            compare_media_by_order(&a, &b, WpApiParamPostsOrderBy::Id, WpApiParamOrder::Asc);
        assert_eq!(result, Some(Ordering::Less));
    }
    #[test]
    fn compare_media_by_id_desc_reverses() {
        let a = MediaBuilder::minimal().with_id(1).build();
        let b = MediaBuilder::minimal().with_id(2).build();
        let result =
            compare_media_by_order(&a, &b, WpApiParamPostsOrderBy::Id, WpApiParamOrder::Desc);
        assert_eq!(result, Some(Ordering::Greater));
    }
    #[test]
    fn compare_media_by_slug() {
        let a = MediaBuilder::minimal().with_slug("apple").build();
        let b = MediaBuilder::minimal().with_slug("banana").build();
        let result =
            compare_media_by_order(&a, &b, WpApiParamPostsOrderBy::Slug, WpApiParamOrder::Asc);
        assert_eq!(result, Some(Ordering::Less));
    }
    #[test]
    fn compare_media_by_title_uses_raw_when_present() {
        let a = MediaBuilder::minimal().with_title("aardvark").build();
        let b = MediaBuilder::minimal().with_title("zebra").build();
        let result =
            compare_media_by_order(&a, &b, WpApiParamPostsOrderBy::Title, WpApiParamOrder::Asc);
        assert_eq!(result, Some(Ordering::Less));
    }
    #[test]
    fn compare_media_by_relevance_returns_none() {
        let a = MediaBuilder::minimal().with_id(1).build();
        let b = MediaBuilder::minimal().with_id(2).build();
        let result = compare_media_by_order(
            &a,
            &b,
            WpApiParamPostsOrderBy::Relevance,
            WpApiParamOrder::Asc,
        );
        assert_eq!(result, None);
    }
}
