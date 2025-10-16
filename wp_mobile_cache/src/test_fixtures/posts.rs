use wp_api::{
    media::MediaId,
    posts::{
        AnyPostWithEditContext, PostContentWithEditContext, PostFootnote, PostGuidWithEditContext,
        PostId, PostMeta, PostStatus, PostTitleWithEditContext, SparsePostExcerpt,
    },
    terms::TermId,
    users::UserId,
};

/// Creates a minimal valid AnyPostWithEditContext for testing
pub fn create_minimal_post() -> AnyPostWithEditContext {
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

/// Creates a fully populated AnyPostWithEditContext for testing
pub fn create_full_post() -> AnyPostWithEditContext {
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
