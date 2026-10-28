//! Making a working copy from a remote post.
//!
//! Every value a working copy holds is converted without loss, so a working
//! copy made from a remote post starts out holding what the server holds.
//! Fields the working copy doesn't model, such as the template, menu order and
//! meta, are not part of it; they stay as they are on the server provided that
//! pushing a working copy sends only the fields the user changed.

use wp_api::{
    date::WpDateString,
    media::MediaId,
    posts::{AnyPostWithEditContext, PostFormat, PostId, PostStatus},
    users::UserId,
};

use crate::db_types::{
    working_copy::{
        FeaturedImage, PageWorkingCopy, PostWorkingCopy, PublishDate, StatusSetting,
        WorkingCopyCommon, WorkingCopyTag,
    },
    working_copy_target::RemotePostReference,
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum RemotePostConversionError {
    #[error("The post's {field} is missing; it must be fetched with the edit context")]
    MissingField { field: String },
    #[error("A post with status {status} can't be edited")]
    UneditableStatus { status: String },
    #[error("A private post can't have a password")]
    PrivatePostWithPassword,
    #[error("Expected post type {expected}, got {actual}")]
    WrongPostType { expected: String, actual: String },
}

impl PostWorkingCopy {
    /// Make a working copy from a post fetched with the edit context.
    pub fn from_remote(post: &AnyPostWithEditContext) -> Result<Self, RemotePostConversionError> {
        expect_post_type(post, "post")?;
        Ok(Self {
            common: common_from_remote(post)?,
            categories: post.categories.clone().unwrap_or_default(),
            tags: post
                .tags
                .iter()
                .flatten()
                .map(|id| WorkingCopyTag::Existing { id: *id })
                .collect(),
            format: post.format.clone().unwrap_or(PostFormat::Standard),
            sticky: post.sticky.unwrap_or(false),
        })
    }
}

impl PageWorkingCopy {
    /// Make a working copy from a page fetched with the edit context.
    pub fn from_remote(post: &AnyPostWithEditContext) -> Result<Self, RemotePostConversionError> {
        expect_post_type(post, "page")?;
        Ok(Self {
            common: common_from_remote(post)?,
            // WordPress uses 0 for a top-level page.
            parent: post.parent.filter(|parent| *parent != PostId(0)),
        })
    }
}

impl RemotePostReference {
    /// The reference for a working copy made from `post`, as it was fetched.
    pub fn from_remote(post: &AnyPostWithEditContext) -> Self {
        Self {
            post_id: post.id,
            base_modified_gmt: post.modified_gmt,
        }
    }
}

/// Make a post working copy from a post fetched with the edit context.
#[uniffi::export]
pub fn post_working_copy_from_remote(
    post: AnyPostWithEditContext,
) -> Result<PostWorkingCopy, RemotePostConversionError> {
    PostWorkingCopy::from_remote(&post)
}

/// Make a page working copy from a page fetched with the edit context.
#[uniffi::export]
pub fn page_working_copy_from_remote(
    page: AnyPostWithEditContext,
) -> Result<PageWorkingCopy, RemotePostConversionError> {
    PageWorkingCopy::from_remote(&page)
}

/// The reference to `post` for a working copy made from it, as it was fetched.
#[uniffi::export]
pub fn remote_post_reference_from_remote(post: AnyPostWithEditContext) -> RemotePostReference {
    RemotePostReference::from_remote(&post)
}

fn expect_post_type(
    post: &AnyPostWithEditContext,
    expected: &'static str,
) -> Result<(), RemotePostConversionError> {
    if post.post_type == expected {
        Ok(())
    } else {
        Err(RemotePostConversionError::WrongPostType {
            expected: expected.to_string(),
            actual: post.post_type.clone(),
        })
    }
}

fn common_from_remote(
    post: &AnyPostWithEditContext,
) -> Result<WorkingCopyCommon, RemotePostConversionError> {
    let content =
        post.content
            .raw
            .clone()
            .ok_or_else(|| RemotePostConversionError::MissingField {
                field: "content.raw".to_string(),
            })?;
    // A post type without title or excerpt support has no such object; one
    // that has it but lacks `raw` wasn't fetched with the edit context.
    let title = match &post.title {
        Some(title) => {
            title
                .raw
                .clone()
                .ok_or_else(|| RemotePostConversionError::MissingField {
                    field: "title.raw".to_string(),
                })?
        }
        None => String::new(),
    };
    let excerpt = match &post.excerpt {
        Some(excerpt) => {
            excerpt
                .raw
                .clone()
                .ok_or_else(|| RemotePostConversionError::MissingField {
                    field: "excerpt.raw".to_string(),
                })?
        }
        None => String::new(),
    };

    Ok(WorkingCopyCommon {
        title,
        content,
        excerpt,
        slug: Some(post.slug.clone()).filter(|slug| !slug.is_empty()),
        status: status_from_remote(&post.status, post.password.as_deref())?,
        publish_date: if is_date_floating(&post.status, &post.date, &post.modified) {
            PublishDate::Immediately
        } else {
            PublishDate::At {
                date: post.date_gmt,
            }
        },
        // WordPress uses 0 for "none" in both fields.
        author: post.author.filter(|author| *author != UserId(0)),
        featured_image: match post.featured_media {
            Some(media_id) if media_id != MediaId(0) => FeaturedImage::Remote { media_id },
            _ => FeaturedImage::None,
        },
        comment_status: post.comment_status.clone(),
        ping_status: post.ping_status.clone(),
    })
}

fn status_from_remote(
    status: &PostStatus,
    password: Option<&str>,
) -> Result<StatusSetting, RemotePostConversionError> {
    // WordPress stores "no password" as an empty string.
    let password = password
        .filter(|password| !password.is_empty())
        .map(str::to_string);
    match status {
        PostStatus::Draft => Ok(StatusSetting::Draft { password }),
        PostStatus::Pending => Ok(StatusSetting::PendingReview { password }),
        // WordPress stores a scheduled post as `future` and turns it back into
        // `publish` once its date passes, so the date alone carries the schedule.
        PostStatus::Publish | PostStatus::Future => Ok(StatusSetting::Publish { password }),
        // WordPress clears the password when saving a private post, so one
        // never comes back. If it did, there'd be nowhere to keep it.
        PostStatus::Private if password.is_none() => Ok(StatusSetting::Private),
        PostStatus::Private => Err(RemotePostConversionError::PrivatePostWithPassword),
        PostStatus::Custom(status) => Ok(StatusSetting::Custom {
            status: status.clone(),
            password,
        }),
        PostStatus::Trash | PostStatus::Any => Err(RemotePostConversionError::UneditableStatus {
            status: status.to_string(),
        }),
    }
}

/// Whether the post's date floats, meaning WordPress sets it when the post is
/// published and the editor shows "Immediately".
///
/// The REST API doesn't report this. This is the block editor's rule from
/// `isEditedPostDateFloating`: an unpublished post whose date equals its
/// modified date.
fn is_date_floating(status: &PostStatus, date: &WpDateString, modified: &WpDateString) -> bool {
    let unpublished = match status {
        PostStatus::Draft | PostStatus::Pending => true,
        PostStatus::Custom(status) => status == "auto-draft",
        _ => false,
    };
    unpublished && date == modified
}

#[cfg(test)]
mod tests {
    use super::*;
    use wp_api::{
        date::WpGmtDateTime,
        posts::{
            PostCommentStatus, PostContentWithEditContext, PostGuidWithEditContext,
            PostTitleWithEditContext, SparsePostExcerpt,
        },
        terms::TermId,
    };

    fn gmt(value: &str) -> WpGmtDateTime {
        value.parse().expect("Date should parse")
    }

    /// A published post as the REST API returns it with the edit context.
    fn remote_post() -> AnyPostWithEditContext {
        AnyPostWithEditContext {
            id: PostId(42),
            date: WpDateString::new("2024-01-15T10:30:00".to_string()),
            date_gmt: gmt("2024-01-15T10:30:00Z"),
            guid: PostGuidWithEditContext {
                raw: Some("https://example.com/?p=42".to_string()),
                rendered: "https://example.com/?p=42".to_string(),
            },
            link: "https://example.com/a-post".to_string(),
            modified: WpDateString::new("2024-01-16T14:20:00".to_string()),
            modified_gmt: gmt("2024-01-16T14:20:00Z"),
            slug: "a-post".to_string(),
            status: PostStatus::Publish,
            post_type: "post".to_string(),
            password: Some(String::new()),
            permalink_template: None,
            generated_slug: None,
            title: Some(PostTitleWithEditContext {
                raw: Some("A post".to_string()),
                rendered: "A post".to_string(),
            }),
            content: PostContentWithEditContext {
                raw: Some("<!-- wp:paragraph --><p>Content</p><!-- /wp:paragraph -->".to_string()),
                rendered: "<p>Content</p>".to_string(),
                protected: Some(false),
                block_version: Some(1),
            },
            author: Some(UserId(10)),
            excerpt: Some(SparsePostExcerpt {
                raw: Some("An excerpt".to_string()),
                rendered: Some("<p>An excerpt</p>".to_string()),
                protected: Some(false),
            }),
            featured_media: Some(MediaId(0)),
            comment_status: Some(PostCommentStatus::Open),
            ping_status: None,
            format: Some(PostFormat::Aside),
            meta: None,
            sticky: Some(false),
            template: String::new(),
            categories: Some(vec![TermId(1)]),
            tags: Some(vec![TermId(7), TermId(8)]),
            parent: None,
            menu_order: None,
            additional_fields: None,
        }
    }

    #[test]
    fn test_published_post_converts_every_field() {
        let working_copy =
            PostWorkingCopy::from_remote(&remote_post()).expect("Conversion should succeed");

        assert_eq!(
            working_copy,
            PostWorkingCopy {
                common: WorkingCopyCommon {
                    title: "A post".to_string(),
                    content: "<!-- wp:paragraph --><p>Content</p><!-- /wp:paragraph -->"
                        .to_string(),
                    excerpt: "An excerpt".to_string(),
                    slug: Some("a-post".to_string()),
                    status: StatusSetting::Publish { password: None },
                    publish_date: PublishDate::At {
                        date: gmt("2024-01-15T10:30:00Z"),
                    },
                    author: Some(UserId(10)),
                    featured_image: FeaturedImage::None,
                    comment_status: Some(PostCommentStatus::Open),
                    ping_status: None,
                },
                categories: vec![TermId(1)],
                tags: vec![
                    WorkingCopyTag::Existing { id: TermId(7) },
                    WorkingCopyTag::Existing { id: TermId(8) },
                ],
                format: PostFormat::Aside,
                sticky: false,
            }
        );
    }

    /// Every status WordPress can store converts with its password, where
    /// the status allows one.
    #[test]
    fn test_status_converts_with_its_password() {
        let secret = || Some("secret".to_string());
        for (status, password, expected) in [
            (
                PostStatus::Draft,
                "secret",
                StatusSetting::Draft { password: secret() },
            ),
            (
                PostStatus::Pending,
                "secret",
                StatusSetting::PendingReview { password: secret() },
            ),
            (
                PostStatus::Publish,
                "secret",
                StatusSetting::Publish { password: secret() },
            ),
            // A scheduled post is `publish` with a future date.
            (
                PostStatus::Future,
                "",
                StatusSetting::Publish { password: None },
            ),
            (PostStatus::Private, "", StatusSetting::Private),
            (
                PostStatus::Custom("in-review".to_string()),
                "secret",
                StatusSetting::Custom {
                    status: "in-review".to_string(),
                    password: secret(),
                },
            ),
        ] {
            let post = AnyPostWithEditContext {
                status: status.clone(),
                password: Some(password.to_string()),
                ..remote_post()
            };

            let common = PostWorkingCopy::from_remote(&post)
                .expect("Conversion should succeed")
                .common;

            assert_eq!(common.status, expected, "status {status:?}");
        }
    }

    #[test]
    fn test_publish_date_follows_the_block_editor_floating_rule() {
        let same_as_modified = WpDateString::new("2024-01-16T14:20:00".to_string());
        for (status, date, expected) in [
            // An unpublished post whose date equals its modified date floats.
            (
                PostStatus::Draft,
                same_as_modified.clone(),
                PublishDate::Immediately,
            ),
            (
                PostStatus::Pending,
                same_as_modified.clone(),
                PublishDate::Immediately,
            ),
            // A draft with a date the user picked keeps it.
            (
                PostStatus::Draft,
                WpDateString::new("2024-01-15T10:30:00".to_string()),
                PublishDate::At {
                    date: gmt("2024-01-15T10:30:00Z"),
                },
            ),
            // A published post's date never floats, even when it equals modified.
            (
                PostStatus::Publish,
                same_as_modified,
                PublishDate::At {
                    date: gmt("2024-01-15T10:30:00Z"),
                },
            ),
        ] {
            let post = AnyPostWithEditContext {
                status: status.clone(),
                date,
                ..remote_post()
            };

            let common = PostWorkingCopy::from_remote(&post)
                .expect("Conversion should succeed")
                .common;

            assert_eq!(common.publish_date, expected, "status {status:?}");
        }
    }

    #[test]
    fn test_page_converts_parent_with_zero_as_top_level() {
        for (parent, expected) in [(Some(PostId(0)), None), (Some(PostId(5)), Some(PostId(5)))] {
            let page = AnyPostWithEditContext {
                post_type: "page".to_string(),
                parent,
                ..remote_post()
            };

            let working_copy =
                PageWorkingCopy::from_remote(&page).expect("Conversion should succeed");

            assert_eq!(working_copy.parent, expected);
        }
    }

    #[test]
    fn test_conversion_refuses_what_it_cannot_represent() {
        let cases = [
            (
                AnyPostWithEditContext {
                    status: PostStatus::Trash,
                    ..remote_post()
                },
                RemotePostConversionError::UneditableStatus {
                    status: "trash".to_string(),
                },
            ),
            (
                AnyPostWithEditContext {
                    content: PostContentWithEditContext {
                        raw: None,
                        ..remote_post().content
                    },
                    ..remote_post()
                },
                RemotePostConversionError::MissingField {
                    field: "content.raw".to_string(),
                },
            ),
            (
                AnyPostWithEditContext {
                    status: PostStatus::Private,
                    password: Some("secret".to_string()),
                    ..remote_post()
                },
                RemotePostConversionError::PrivatePostWithPassword,
            ),
            (
                AnyPostWithEditContext {
                    post_type: "page".to_string(),
                    ..remote_post()
                },
                RemotePostConversionError::WrongPostType {
                    expected: "post".to_string(),
                    actual: "page".to_string(),
                },
            ),
        ];

        for (post, expected) in cases {
            assert_eq!(PostWorkingCopy::from_remote(&post), Err(expected));
        }
    }

    #[test]
    fn test_reference_records_the_remote_post_and_its_modified_date() {
        assert_eq!(
            RemotePostReference::from_remote(&remote_post()),
            RemotePostReference {
                post_id: PostId(42),
                base_modified_gmt: gmt("2024-01-16T14:20:00Z"),
            }
        );
    }
}
