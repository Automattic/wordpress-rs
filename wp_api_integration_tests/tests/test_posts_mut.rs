use macro_helper::{generate_update_post_format_test, generate_update_test};
use serial_test::serial;
use wp_api::posts::{
    PostCommentStatus, PostCreateParams, PostFormat, PostPingStatus, PostStatus, PostUpdateParams,
    PostWithEditContext,
};
use wp_api_integration_tests::{
    api_client,
    backend::{Backend, RestoreServer},
    AssertResponse, CATEGORY_ID_1, FIRST_POST_ID, MEDIA_ID_611, POST_TEMPLATE_SINGLE_WITH_SIDEBAR,
    SECOND_USER_ID, TAG_ID_100,
};
use wp_cli::WpCliPost;

#[tokio::test]
#[serial]
async fn create_post_with_just_title() {
    test_create_post(
        &PostCreateParams {
            title: Some("foo".to_string()),
            ..Default::default()
        },
        |created_post, post_from_wp_cli| {
            assert_eq!(created_post.title.raw, "foo");
            assert_eq!(post_from_wp_cli.title, "foo");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_post_with_just_content() {
    test_create_post(
        &PostCreateParams {
            content: Some("foo".to_string()),
            ..Default::default()
        },
        |created_post, post_from_wp_cli| {
            assert_eq!(created_post.content.raw, "foo");
            assert_eq!(post_from_wp_cli.content, "foo");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_post_with_just_excerpt() {
    test_create_post(
        &PostCreateParams {
            excerpt: Some("foo".to_string()),
            ..Default::default()
        },
        |created_post, post_from_wp_cli| {
            assert_eq!(created_post.excerpt.raw, "foo");
            assert_eq!(post_from_wp_cli.excerpt, "foo");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_post_with_title_content_and_excerpt() {
    test_create_post(
        &PostCreateParams {
            title: Some("foo".to_string()),
            content: Some("bar".to_string()),
            excerpt: Some("baz".to_string()),
            ..Default::default()
        },
        |created_post, post_from_wp_cli| {
            assert_eq!(created_post.title.raw, "foo");
            assert_eq!(post_from_wp_cli.title, "foo");
            assert_eq!(created_post.content.raw, "bar");
            assert_eq!(post_from_wp_cli.content, "bar");
            assert_eq!(created_post.excerpt.raw, "baz");
            assert_eq!(post_from_wp_cli.excerpt, "baz");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn delete_post() {
    // Delete the post using the API and ensure it's successful
    let post_delete_response = api_client().posts().delete(&FIRST_POST_ID).await;
    assert!(post_delete_response.is_ok(), "{:#?}", post_delete_response);

    // Assert that the post was deleted
    assert!(
        !Backend::posts(None)
            .await
            .into_iter()
            .any(|u| u.id == FIRST_POST_ID.0 as i64),
        "Post wasn't deleted"
    );

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn trash_post() {
    // Trash the post using the API and ensure it's successful
    let post_trash_response = api_client().posts().trash(&FIRST_POST_ID).await;
    assert!(post_trash_response.is_ok(), "{:#?}", post_trash_response);

    // Assert that the post was trashed
    let trashed_post = Backend::posts(Some("trash"))
        .await
        .into_iter()
        .find(|u| u.id == FIRST_POST_ID.0 as i64);
    assert!(trashed_post.is_some(), "Can't find the trashed post");
    assert_eq!(
        trashed_post.unwrap().post_status,
        "trash",
        "Post wasn't trashed"
    );

    RestoreServer::db().await;
}

generate_update_test!(
    update_date,
    date,
    "2024-09-09T12:00:00".to_string(),
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.date, "2024-09-09T12:00:00");
        assert_eq!(updated_post_from_wp_cli.date, "2024-09-09 12:00:00");
    }
);

generate_update_test!(
    update_date_gmt,
    date_gmt,
    "2024-09-09T12:00:00".to_string(),
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.date_gmt, "2024-09-09T12:00:00");
        assert_eq!(updated_post_from_wp_cli.date_gmt, "2024-09-09 12:00:00");
    }
);

generate_update_test!(
    update_slug,
    slug,
    "new_slug".to_string(),
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.slug, "new_slug");
        assert_eq!(updated_post_from_wp_cli.slug, "new_slug");
    }
);

generate_update_test!(
    update_status_to_draft,
    status,
    PostStatus::Draft,
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.status, PostStatus::Draft);
        assert_eq!(
            updated_post_from_wp_cli.post_status,
            PostStatus::Draft.as_str()
        );
    }
);

generate_update_test!(
    update_password,
    password,
    "new_password".to_string(),
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.password, "new_password");
        assert_eq!(updated_post_from_wp_cli.password, "new_password");
    }
);

generate_update_test!(
    update_title,
    title,
    "new_title".to_string(),
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.title.raw, "new_title");
        assert_eq!(updated_post_from_wp_cli.title, "new_title");
    }
);

generate_update_test!(
    update_content,
    content,
    "new_content".to_string(),
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.content.raw, "new_content");
        assert_eq!(updated_post_from_wp_cli.content, "new_content");
    }
);

generate_update_test!(
    update_author,
    author,
    SECOND_USER_ID,
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.author, SECOND_USER_ID);
        assert_eq!(updated_post_from_wp_cli.author, SECOND_USER_ID.0 as i64);
    }
);

generate_update_test!(
    update_excerpt,
    excerpt,
    "new_excerpt".to_string(),
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.excerpt.raw, "new_excerpt");
        assert_eq!(updated_post_from_wp_cli.excerpt, "new_excerpt");
    }
);

generate_update_test!(
    update_featured_media,
    featured_media,
    MEDIA_ID_611,
    |updated_post, _| {
        assert_eq!(updated_post.featured_media, MEDIA_ID_611);
    }
);

generate_update_test!(
    update_comment_status_to_open,
    comment_status,
    PostCommentStatus::Open,
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.comment_status, PostCommentStatus::Open);
        assert_eq!(
            updated_post_from_wp_cli.comment_status,
            PostCommentStatus::Open.as_str()
        );
    }
);

generate_update_test!(
    update_comment_status_to_closed,
    comment_status,
    PostCommentStatus::Closed,
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.comment_status, PostCommentStatus::Closed);
        assert_eq!(
            updated_post_from_wp_cli.comment_status,
            PostCommentStatus::Closed.as_str()
        );
    }
);

generate_update_test!(
    update_ping_status_to_open,
    ping_status,
    PostPingStatus::Open,
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.ping_status, PostPingStatus::Open);
        assert_eq!(
            updated_post_from_wp_cli.ping_status,
            PostPingStatus::Open.as_str()
        );
    }
);

generate_update_test!(
    update_ping_status_to_closed,
    ping_status,
    PostPingStatus::Closed,
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.ping_status, PostPingStatus::Closed);
        assert_eq!(
            updated_post_from_wp_cli.ping_status,
            PostPingStatus::Closed.as_str()
        );
    }
);

generate_update_test!(
    update_template,
    template,
    POST_TEMPLATE_SINGLE_WITH_SIDEBAR.to_string(),
    |updated_post, _| {
        assert_eq!(updated_post.template, POST_TEMPLATE_SINGLE_WITH_SIDEBAR);
    }
);

#[tokio::test]
#[serial]
async fn update_sticky_to_true() {
    test_update_post(
        &PostUpdateParams {
            sticky: Some(true),
            ..Default::default()
        },
        |updated_post, _| {
            assert!(updated_post.sticky);
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn update_sticky_to_false() {
    test_update_post(
        &PostUpdateParams {
            sticky: Some(false),
            ..Default::default()
        },
        |updated_post, _| {
            assert!(!updated_post.sticky);
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn update_categories() {
    let updated_value = vec![CATEGORY_ID_1];
    test_update_post(
        &PostUpdateParams {
            categories: updated_value.clone(),
            ..Default::default()
        },
        |updated_post, _| {
            assert_eq!(updated_post.categories, updated_value);
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn update_tags() {
    let updated_value = vec![TAG_ID_100];
    test_update_post(
        &PostUpdateParams {
            tags: updated_value.clone(),
            ..Default::default()
        },
        |updated_post, _| {
            assert_eq!(updated_post.tags, updated_value);
        },
    )
    .await;
}

generate_update_post_format_test!(Standard);
generate_update_post_format_test!(Aside);
generate_update_post_format_test!(Chat);
generate_update_post_format_test!(Gallery);
generate_update_post_format_test!(Link);
generate_update_post_format_test!(Image);
generate_update_post_format_test!(Quote);
generate_update_post_format_test!(Status);
generate_update_post_format_test!(Video);
generate_update_post_format_test!(Audio);

async fn test_create_post<F>(params: &PostCreateParams, assert: F)
where
    F: Fn(PostWithEditContext, WpCliPost),
{
    let created_post = api_client().posts().create(params).await.assert_response();
    let created_post_from_wp_cli = Backend::post(&created_post.id).await;
    assert(created_post, created_post_from_wp_cli);
    RestoreServer::db().await;
}

async fn test_update_post<F>(params: &PostUpdateParams, assert: F)
where
    F: Fn(PostWithEditContext, WpCliPost),
{
    let updated_post = api_client()
        .posts()
        .update(&FIRST_POST_ID, params)
        .await
        .assert_response();
    let updated_post_from_wp_cli = Backend::post(&FIRST_POST_ID).await;
    assert(updated_post, updated_post_from_wp_cli);
    RestoreServer::db().await;
}

mod macro_helper {
    macro_rules! generate_update_test {
        ($ident:ident, $field:ident, $new_value:expr, $assertion:expr) => {
            paste::paste! {
                #[tokio::test]
                #[serial]
                async fn $ident() {
                    let updated_value = $new_value;
                    test_update_post(
                        &PostUpdateParams {
                            $field: Some(updated_value),
                            ..Default::default()
                        }, $assertion)
                    .await;
                }
            }
        };
    }

    macro_rules! generate_update_post_format_test {
        ($format:ident) => {
            paste::paste! {
                #[tokio::test]
                #[serial]
                async fn [<update_post_format_to_ $format:lower>]() {
                    test_update_post(
                        &PostUpdateParams {
                            format: Some(PostFormat::$format),
                            ..Default::default()
                        },
                        |updated_post, _| {
                            assert_eq!(updated_post.format, PostFormat::$format);
                        }
                    ).await;
                }
            }
        };
    }

    pub(super) use generate_update_post_format_test;
    pub(super) use generate_update_test;
}
