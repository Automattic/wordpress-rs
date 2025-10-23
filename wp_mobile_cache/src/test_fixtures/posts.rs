use std::sync::atomic::{AtomicI64, Ordering};
use wp_api::{
    media::MediaId,
    posts::{
        AnyPostWithEditContext, PostContentWithEditContext, PostFootnote, PostGuidWithEditContext,
        PostId, PostMeta, PostStatus, PostTitleWithEditContext, SparsePostExcerpt,
    },
    terms::TermId,
    users::UserId,
};

/// Initial state for PostBuilder - determines which field values are populated.
pub enum PostBuilderInitialState {
    /// Minimal valid post with only required fields populated
    Minimal,
    /// Fully populated post with all optional fields set
    Full,
}

/// Builder for creating test posts with automatic ID management.
///
/// Use `PostBuilder::minimal()` for posts with only required fields,
/// or `PostBuilder::full()` for posts with all fields populated.
///
/// Reduces boilerplate and prevents ID collisions in tests by auto-incrementing IDs.
///
/// # Example
///
/// ```rust
/// // Minimal post with custom fields
/// let post1 = PostBuilder::minimal()
///     .with_author(UserId(10))
///     .build();
///
/// // Full post with overrides
/// let post2 = PostBuilder::full()
///     .with_status(PostStatus::Draft)
///     .build();
///
/// // IDs are automatically unique (1000, 1001, ...)
/// ```
pub struct PostBuilder {
    post: AnyPostWithEditContext,
}

impl PostBuilder {
    /// Create a new builder with auto-incremented ID starting from 1000.
    ///
    /// Uses thread-safe atomic counter to ensure unique IDs across tests.
    ///
    /// **Note**: Prefer using `PostBuilder::minimal()` or `PostBuilder::full()`
    /// instead of calling this method directly.
    pub fn new(initial_state: PostBuilderInitialState) -> Self {
        static COUNTER: AtomicI64 = AtomicI64::new(1000);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);

        let mut post = match initial_state {
            PostBuilderInitialState::Minimal => create_minimal_post(),
            PostBuilderInitialState::Full => create_full_post(),
        };
        post.id = PostId(id);
        Self { post }
    }

    /// Create a minimal post builder with only required fields populated.
    ///
    /// This is the most common starting point for test posts.
    pub fn minimal() -> Self {
        Self::new(PostBuilderInitialState::Minimal)
    }

    /// Create a full post builder with all optional fields populated.
    ///
    /// Useful for testing complete post serialization/deserialization.
    pub fn full() -> Self {
        Self::new(PostBuilderInitialState::Full)
    }

    /// Set a specific post ID (overrides auto-increment).
    pub fn with_id(mut self, id: i64) -> Self {
        self.post.id = PostId(id);
        self
    }

    /// Set the post author.
    pub fn with_author(mut self, author: UserId) -> Self {
        self.post.author = Some(author);
        self
    }

    /// Set the post status.
    pub fn with_status(mut self, status: PostStatus) -> Self {
        self.post.status = status;
        self
    }

    /// Set the post title.
    pub fn with_title(mut self, title: &str) -> Self {
        self.post.title.rendered = title.to_string();
        self.post.title.raw = Some(title.to_string());
        self
    }

    /// Set the post slug.
    pub fn with_slug(mut self, slug: &str) -> Self {
        self.post.slug = slug.to_string();
        self
    }

    /// Set post categories.
    pub fn with_categories(mut self, categories: Vec<TermId>) -> Self {
        self.post.categories = Some(categories);
        self
    }

    /// Set post tags.
    pub fn with_tags(mut self, tags: Vec<TermId>) -> Self {
        self.post.tags = Some(tags);
        self
    }

    /// Set both categories and tags.
    pub fn with_terms(mut self, categories: Vec<TermId>, tags: Vec<TermId>) -> Self {
        self.post.categories = Some(categories);
        self.post.tags = Some(tags);
        self
    }

    /// Set featured media.
    pub fn with_featured_media(mut self, media_id: MediaId) -> Self {
        self.post.featured_media = Some(media_id);
        self
    }

    /// Set parent post.
    pub fn with_parent(mut self, parent_id: PostId) -> Self {
        self.post.parent = Some(parent_id);
        self
    }

    /// Set sticky status.
    pub fn with_sticky(mut self, sticky: bool) -> Self {
        self.post.sticky = Some(sticky);
        self
    }

    /// Build the final AnyPostWithEditContext.
    pub fn build(self) -> AnyPostWithEditContext {
        self.post
    }
}

impl Default for PostBuilder {
    fn default() -> Self {
        Self::minimal()
    }
}

fn create_minimal_post() -> AnyPostWithEditContext {
    AnyPostWithEditContext {
        id: PostId(1),
        date: "2024-01-01T00:00:00".to_string(),
        date_gmt: "2024-01-01T00:00:00Z".parse().unwrap(),
        guid: PostGuidWithEditContext {
            raw: None,
            rendered: "https://example.com/?p=1".to_string(),
        },
        link: "https://example.com/minimal-post".to_string(),
        modified: "2024-01-01T00:00:00".to_string(),
        modified_gmt: "2024-01-01T00:00:00Z".parse().unwrap(),
        slug: "minimal-post".to_string(),
        status: PostStatus::Publish,
        post_type: "post".to_string(),
        password: "".to_string(),
        permalink_template: None,
        generated_slug: None,
        title: PostTitleWithEditContext {
            raw: None,
            rendered: "Minimal Post".to_string(),
        },
        content: PostContentWithEditContext {
            raw: None,
            rendered: "<p>Content</p>".to_string(),
            protected: None,
            block_version: None,
        },
        author: None,
        excerpt: None,
        featured_media: None,
        comment_status: None,
        ping_status: None,
        format: None,
        meta: None,
        sticky: None,
        template: "".to_string(),
        categories: None,
        tags: None,
        parent: None,
        menu_order: None,
    }
}

fn create_full_post() -> AnyPostWithEditContext {
    AnyPostWithEditContext {
        id: PostId(42),
        date: "2024-01-15T10:30:00".to_string(),
        date_gmt: "2024-01-15T10:30:00Z".parse().unwrap(),
        guid: PostGuidWithEditContext {
            raw: Some("https://example.com/?p=42".to_string()),
            rendered: "https://example.com/?p=42".to_string(),
        },
        link: "https://example.com/full-post".to_string(),
        modified: "2024-01-16T14:20:00".to_string(),
        modified_gmt: "2024-01-16T14:20:00Z".parse().unwrap(),
        slug: "full-post".to_string(),
        status: PostStatus::Draft,
        post_type: "post".to_string(),
        password: "secret".to_string(),
        permalink_template: Some("https://example.com/%postname%/".to_string()),
        generated_slug: Some("full-post-123".to_string()),
        title: PostTitleWithEditContext {
            raw: Some("Full Post Title".to_string()),
            rendered: "Full Post Title".to_string(),
        },
        content: PostContentWithEditContext {
            raw: Some("<!-- wp:paragraph --><p>Content</p><!-- /wp:paragraph -->".to_string()),
            rendered: "<p>Content</p>".to_string(),
            protected: Some(false),
            block_version: Some(1),
        },
        author: Some(UserId(10)),
        excerpt: Some(SparsePostExcerpt {
            raw: Some("Excerpt raw".to_string()),
            rendered: Some("<p>Excerpt</p>".to_string()),
            protected: Some(false),
        }),
        featured_media: Some(MediaId(100)),
        comment_status: Some(wp_api::posts::PostCommentStatus::Open),
        ping_status: Some(wp_api::posts::PostPingStatus::Closed),
        format: Some(wp_api::posts::PostFormat::Standard),
        meta: Some(PostMeta {
            footnotes: Some(vec![
                PostFootnote {
                    id: "fn1".to_string(),
                    content: "Footnote 1".to_string(),
                },
                PostFootnote {
                    id: "fn2".to_string(),
                    content: "Footnote 2".to_string(),
                },
            ]),
        }),
        sticky: Some(true),
        template: "custom-template.php".to_string(),
        categories: Some(vec![TermId(1), TermId(2), TermId(3)]),
        tags: Some(vec![TermId(10), TermId(20)]),
        parent: Some(PostId(5)),
        menu_order: Some(3),
    }
}
