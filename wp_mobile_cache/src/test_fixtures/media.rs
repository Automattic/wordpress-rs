use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use wp_api::{
    date::WpDateString,
    media::{
        MediaCaptionWithEditContext, MediaDescriptionWithEditContext, MediaDetails, MediaId,
        MediaStatus, MediaType, MediaWithEditContext,
    },
    posts::{
        PostCommentStatus, PostGuidWithEditContext, PostId, PostPingStatus,
        PostTitleWithEditContext,
    },
    users::UserId,
};

/// Initial state for MediaBuilder - determines which field values are populated.
pub enum MediaBuilderInitialState {
    /// Minimal valid media with only required fields populated
    Minimal,
    /// Fully populated media with all optional fields set
    Full,
}

/// Builder for creating test media entities with automatic ID management.
///
/// Use `MediaBuilder::minimal()` for media with only required fields,
/// or `MediaBuilder::full()` for media with all fields populated.
///
/// Reduces boilerplate and prevents ID collisions in tests by auto-incrementing IDs.
pub struct MediaBuilder {
    media: MediaWithEditContext,
}

impl MediaBuilder {
    /// Create a new builder with auto-incremented ID starting from 2000.
    ///
    /// Uses thread-safe atomic counter to ensure unique IDs across tests.
    /// IDs start at 2000 to make them distinguishable from PostBuilder IDs (1000+).
    pub fn new(initial_state: MediaBuilderInitialState) -> Self {
        static COUNTER: AtomicI64 = AtomicI64::new(2000);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);

        let mut media = match initial_state {
            MediaBuilderInitialState::Minimal => create_minimal_media(),
            MediaBuilderInitialState::Full => create_full_media(),
        };
        media.id = MediaId(id);
        Self { media }
    }

    /// Create a minimal media builder with only required fields populated.
    pub fn minimal() -> Self {
        Self::new(MediaBuilderInitialState::Minimal)
    }

    /// Create a full media builder with all optional fields populated.
    pub fn full() -> Self {
        Self::new(MediaBuilderInitialState::Full)
    }

    /// Set a specific media ID (overrides auto-increment).
    pub fn with_id(mut self, id: i64) -> Self {
        self.media.id = MediaId(id);
        self
    }

    /// Set a specific media ID (overrides auto-increment).
    pub fn with_media_id(mut self, media_id: MediaId) -> Self {
        self.media.id = media_id;
        self
    }

    /// Set the media slug.
    pub fn with_slug(mut self, slug: &str) -> Self {
        self.media.slug = slug.into();
        self
    }

    /// Set the media status.
    pub fn with_status(mut self, status: MediaStatus) -> Self {
        self.media.status = status;
        self
    }

    /// Set the media title.
    pub fn with_title(mut self, title: &str) -> Self {
        self.media.title = PostTitleWithEditContext {
            raw: Some(title.into()),
            rendered: title.into(),
        };
        self
    }

    /// Set the media author.
    pub fn with_author(mut self, author: UserId) -> Self {
        self.media.author = author;
        self
    }

    /// Set the attached post id.
    pub fn with_post_id(mut self, post_id: PostId) -> Self {
        self.media.post_id = Some(post_id);
        self
    }

    /// Set the media type.
    pub fn with_media_type(mut self, media_type: MediaType) -> Self {
        self.media.media_type = media_type;
        self
    }

    /// Set the MIME type.
    pub fn with_mime_type(mut self, mime: &str) -> Self {
        self.media.mime_type = mime.into();
        self
    }

    /// Build the final MediaWithEditContext.
    pub fn build(self) -> MediaWithEditContext {
        self.media
    }
}

impl Default for MediaBuilder {
    fn default() -> Self {
        Self::minimal()
    }
}

fn create_minimal_media() -> MediaWithEditContext {
    MediaWithEditContext {
        id: MediaId(0),
        date: WpDateString("2026-01-01T00:00:00".to_string()),
        date_gmt: "2026-01-01T00:00:00Z".parse().unwrap(),
        guid: PostGuidWithEditContext {
            raw: None,
            rendered: "https://example.com/?p=0".into(),
        },
        link: "https://example.com/0".into(),
        modified: WpDateString("2026-01-01T00:00:00".to_string()),
        modified_gmt: "2026-01-01T00:00:00Z".parse().unwrap(),
        slug: "media-0".into(),
        status: MediaStatus::Inherit,
        post_type: "attachment".into(),
        password: None,
        permalink_template: "https://example.com/?attachment_id=0".into(),
        generated_slug: "media-0".into(),
        title: PostTitleWithEditContext {
            raw: None,
            rendered: "Media 0".into(),
        },
        author: UserId(1),
        comment_status: PostCommentStatus::Open,
        ping_status: PostPingStatus::Open,
        template: String::new(),
        alt_text: String::new(),
        caption: MediaCaptionWithEditContext {
            raw: String::new(),
            rendered: String::new(),
        },
        description: MediaDescriptionWithEditContext {
            raw: String::new(),
            rendered: String::new(),
        },
        media_type: MediaType::File,
        mime_type: "application/octet-stream".into(),
        media_details: Arc::new(MediaDetails {
            payload: serde_json::value::RawValue::from_string("{}".into()).unwrap(),
        }),
        post_id: None,
        source_url: "https://example.com/media-0.bin".into(),
        missing_image_sizes: Vec::new(),
    }
}

fn create_full_media() -> MediaWithEditContext {
    let mut m = create_minimal_media();
    m.password = Some("secret".into());
    m.post_id = Some(PostId(100));
    m.alt_text = "alt text".into();
    m.media_type = MediaType::Image;
    m.mime_type = "image/jpeg".into();
    m.caption = MediaCaptionWithEditContext {
        raw: "caption raw".into(),
        rendered: "<p>caption rendered</p>".into(),
    };
    m.description = MediaDescriptionWithEditContext {
        raw: "description raw".into(),
        rendered: "<p>description rendered</p>".into(),
    };
    m.missing_image_sizes = vec!["thumbnail".into(), "medium".into()];
    m.media_details = Arc::new(MediaDetails {
        payload: serde_json::value::RawValue::from_string(
            r#"{"filesize":12345,"width":1024,"height":768}"#.into(),
        )
        .unwrap(),
    });
    m.title = PostTitleWithEditContext {
        raw: Some("Full Media Title".into()),
        rendered: "Full Media Title".into(),
    };
    m.guid = PostGuidWithEditContext {
        raw: Some("https://example.com/?p=999".into()),
        rendered: "https://example.com/?p=999".into(),
    };
    m
}
