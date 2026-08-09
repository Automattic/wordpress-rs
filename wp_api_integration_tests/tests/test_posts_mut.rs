use macro_helper::{
    generate_update_post_format_test, generate_update_post_status_test, generate_update_test,
};
use std::collections::HashMap;
use wp_api::date::WpDateString;
use wp_api::posts::{
    AnyPostWithEditContext, PostCommentStatus, PostCreateParams, PostFootnote, PostFormat,
    PostListParams, PostMeta, PostPingStatus, PostRetrieveParams, PostStatus, PostUpdateParams,
};
use wp_api::request::endpoint::posts_endpoint::PostEndpointType;
use wp_api::terms::TermId;
use wp_api::{JsonValue, WpAdditionalFields};
use wp_api_integration_tests::prelude::*;
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
            assert_eq!(
                created_post.title.and_then(|t| t.raw),
                Some("foo".to_string())
            );
            assert_eq!(post_from_wp_cli.title, "foo");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_post_with_title_and_meta() {
    let params = PostCreateParams {
        title: Some("foo".to_string()),
        meta: Some(
            PostMeta::new()
                .with_footnotes(vec![PostFootnote {
                    id: "bar".to_string(),
                    content: "baz".to_string(),
                }])
                .unwrap(),
        ),
        ..Default::default()
    };
    let created_post = api_client()
        .posts()
        .create(&PostEndpointType::Posts, &params)
        .await
        .assert_response()
        .data;
    // Bind the id first so it remains available after the response fields are consumed below.
    let created_post_id = created_post.id;
    let created_post_from_wp_cli = Backend::post(&created_post_id).await;

    // Assertions on the create response.
    assert_eq!(
        created_post.title.and_then(|t| t.raw),
        Some("foo".to_string())
    );
    assert_eq!(created_post_from_wp_cli.title, "foo");
    {
        let meta = created_post.meta.unwrap();
        let footnotes = meta.footnotes().unwrap();
        let footnote = footnotes.first().unwrap();
        assert_eq!(footnote.id, "bar");
        assert_eq!(footnote.content, "baz");
    }

    // Re-fetch via REST GET and assert footnotes round-trip end-to-end.
    // The POST response can echo the request body without persisting it;
    // the GET on the same id observes what actually landed in wp_postmeta.
    let fetched = api_client()
        .posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Posts,
            &created_post_id,
            &PostRetrieveParams::default(),
        )
        .await
        .assert_response()
        .data;
    let meta = fetched.meta.unwrap();
    let footnotes = meta.footnotes().unwrap();
    let footnote = footnotes.first().unwrap();
    assert_eq!(footnote.id, "bar");
    assert_eq!(footnote.content, "baz");

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn update_post_meta_round_trips_single_key_via_with_value() {
    let params = PostUpdateParams {
        meta: Some(PostMeta::new().with_value(
            "wp_rs_test_string".to_string(),
            JsonValue::String("hello".to_string()),
        )),
        ..Default::default()
    };
    api_client()
        .posts()
        .update(&PostEndpointType::Posts, &FIRST_POST_ID, &params)
        .await
        .assert_response();

    // Re-fetch and verify the value landed in wp_postmeta.
    // The PATCH response can echo the request body without persisting it;
    // the GET on the same id observes what actually persisted.
    let fetched = api_client()
        .posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Posts,
            &FIRST_POST_ID,
            &PostRetrieveParams::default(),
        )
        .await
        .assert_response()
        .data;
    let meta = fetched.meta.unwrap();
    assert_eq!(
        meta.value_for_key("wp_rs_test_string"),
        Some(JsonValue::String("hello".to_string()))
    );

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn update_post_meta_round_trips_two_keys_via_chained_with_value() {
    let params = PostUpdateParams {
        meta: Some(
            PostMeta::new()
                .with_value(
                    "wp_rs_test_string".to_string(),
                    JsonValue::String("hello".to_string()),
                )
                .with_value("wp_rs_test_number".to_string(), JsonValue::Int(42)),
        ),
        ..Default::default()
    };
    api_client()
        .posts()
        .update(&PostEndpointType::Posts, &FIRST_POST_ID, &params)
        .await
        .assert_response();

    // Re-fetch and verify both values landed in wp_postmeta.
    // The PATCH response can echo the request body without persisting it;
    // the GET on the same id observes what actually persisted.
    let fetched = api_client()
        .posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Posts,
            &FIRST_POST_ID,
            &PostRetrieveParams::default(),
        )
        .await
        .assert_response()
        .data;
    let meta = fetched.meta.unwrap();
    assert_eq!(
        meta.value_for_key("wp_rs_test_string"),
        Some(JsonValue::String("hello".to_string()))
    );
    assert_eq!(
        meta.value_for_key("wp_rs_test_number"),
        Some(JsonValue::Int(42))
    );

    RestoreServer::db().await;
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
            assert_eq!(created_post.content.raw, Some("foo".to_string()));
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
            assert_eq!(created_post.excerpt.unwrap().raw, Some("foo".to_string()));
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
            assert_eq!(
                created_post.title.and_then(|t| t.raw),
                Some("foo".to_string())
            );
            assert_eq!(post_from_wp_cli.title, "foo");
            assert_eq!(created_post.content.raw, Some("bar".to_string()));
            assert_eq!(post_from_wp_cli.content, "bar");
            assert_eq!(created_post.excerpt.unwrap().raw, Some("baz".to_string()));
            assert_eq!(post_from_wp_cli.excerpt, "baz");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn delete_post() {
    // Delete the post using the API and ensure it's successful
    let post_delete_response = api_client()
        .posts()
        .delete(&PostEndpointType::Posts, &FIRST_POST_ID)
        .await;
    assert!(post_delete_response.is_ok(), "{post_delete_response:#?}");
    assert!(post_delete_response.unwrap().data.deleted);

    // Assert that the post was deleted
    assert!(
        !Backend::posts(None)
            .await
            .into_iter()
            .any(|u| u.id == FIRST_POST_ID.0),
        "Post wasn't deleted"
    );

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn trash_post() {
    // Trash the post using the API and ensure it's successful
    let post_trash_response = api_client()
        .posts()
        .trash(&PostEndpointType::Posts, &FIRST_POST_ID)
        .await;
    assert!(post_trash_response.is_ok(), "{post_trash_response:#?}");

    // Assert that the post was trashed
    let trashed_post = Backend::posts(Some("trash"))
        .await
        .into_iter()
        .find(|u| u.id == FIRST_POST_ID.0);
    assert!(trashed_post.is_some(), "Can't find the trashed post");
    assert_eq!(
        trashed_post.unwrap().post_status,
        PostStatus::Trash.to_string(),
        "Post wasn't trashed"
    );

    RestoreServer::db().await;
}

generate_update_test!(
    update_date,
    date,
    WpDateString("2024-09-09T12:00:00".to_string()),
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.date.0, "2024-09-09T12:00:00");
        assert_eq!(updated_post_from_wp_cli.date, "2024-09-09 12:00:00");
    }
);

generate_update_test!(
    update_date_gmt,
    date_gmt,
    unwrapped_wp_gmt_date_time("2024-09-09T12:00:00+0000"),
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(
            updated_post.date_gmt,
            unwrapped_wp_gmt_date_time("2024-09-09T12:00:00+0000")
        );
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
    update_password,
    password,
    "new_password".to_string(),
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.password, Some("new_password".to_string()));
        assert_eq!(updated_post_from_wp_cli.password, "new_password");
    }
);

generate_update_test!(
    update_title,
    title,
    "new_title".to_string(),
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(
            updated_post.title.and_then(|t| t.raw),
            Some("new_title".to_string())
        );
        assert_eq!(updated_post_from_wp_cli.title, "new_title");
    }
);

generate_update_test!(
    update_content,
    content,
    "new_content".to_string(),
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.content.raw, Some("new_content".to_string()));
        assert_eq!(updated_post_from_wp_cli.content, "new_content");
    }
);

generate_update_test!(
    update_author,
    author,
    SECOND_USER_ID,
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.author, Some(SECOND_USER_ID));
        assert_eq!(updated_post_from_wp_cli.author, SECOND_USER_ID.0);
    }
);

generate_update_test!(
    update_excerpt,
    excerpt,
    "new_excerpt".to_string(),
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(
            updated_post.excerpt.unwrap().raw,
            Some("new_excerpt".to_string())
        );
        assert_eq!(updated_post_from_wp_cli.excerpt, "new_excerpt");
    }
);

generate_update_test!(
    update_featured_media,
    featured_media,
    MEDIA_ID_611,
    |updated_post, _| {
        assert_eq!(updated_post.featured_media, Some(MEDIA_ID_611));
    }
);

generate_update_test!(
    update_comment_status_to_open,
    comment_status,
    PostCommentStatus::Open,
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.comment_status, Some(PostCommentStatus::Open));
        assert_eq!(
            updated_post_from_wp_cli.comment_status,
            PostCommentStatus::Open.to_string()
        );
    }
);

generate_update_test!(
    update_comment_status_to_closed,
    comment_status,
    PostCommentStatus::Closed,
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.comment_status, Some(PostCommentStatus::Closed));
        assert_eq!(
            updated_post_from_wp_cli.comment_status,
            PostCommentStatus::Closed.to_string()
        );
    }
);

generate_update_test!(
    update_ping_status_to_open,
    ping_status,
    PostPingStatus::Open,
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.ping_status, Some(PostPingStatus::Open));
        assert_eq!(
            updated_post_from_wp_cli.ping_status,
            PostPingStatus::Open.to_string()
        );
    }
);

generate_update_test!(
    update_ping_status_to_closed,
    ping_status,
    PostPingStatus::Closed,
    |updated_post, updated_post_from_wp_cli| {
        assert_eq!(updated_post.ping_status, Some(PostPingStatus::Closed));
        assert_eq!(
            updated_post_from_wp_cli.ping_status,
            PostPingStatus::Closed.to_string()
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
async fn update_meta_to_add_footnote() {
    let params = PostUpdateParams {
        meta: Some(
            PostMeta::new()
                .with_footnotes(vec![PostFootnote {
                    id: "foo".to_string(),
                    content: "bar".to_string(),
                }])
                .unwrap(),
        ),
        ..Default::default()
    };
    let updated_post = api_client()
        .posts()
        .update(&PostEndpointType::Posts, &FIRST_POST_ID, &params)
        .await
        .assert_response()
        .data;

    // Assertion on the update response.
    {
        let meta = updated_post.meta.unwrap();
        let footnotes = meta.footnotes().unwrap();
        let footnote = footnotes.first().unwrap();
        assert_eq!(footnote.id, "foo");
        assert_eq!(footnote.content, "bar");
    }

    // Re-fetch via REST GET and assert footnotes round-trip end-to-end.
    // The PATCH response can echo the request body without persisting it;
    // the GET on the same id observes what actually landed in wp_postmeta.
    let fetched = api_client()
        .posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Posts,
            &FIRST_POST_ID,
            &PostRetrieveParams::default(),
        )
        .await
        .assert_response()
        .data;
    let meta = fetched.meta.unwrap();
    let footnotes = meta.footnotes().unwrap();
    let footnote = footnotes.first().unwrap();
    assert_eq!(footnote.id, "foo");
    assert_eq!(footnote.content, "bar");

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn update_sticky_to_true() {
    test_update_post(
        &PostUpdateParams {
            sticky: Some(true),
            ..Default::default()
        },
        |updated_post, _| {
            assert_eq!(updated_post.sticky, Some(true));
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
            assert_eq!(updated_post.sticky, Some(false));
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn update_categories() {
    let updated_value = vec![CATEGORY_ID_59];
    test_update_post(
        &PostUpdateParams {
            categories: updated_value.clone(),
            ..Default::default()
        },
        |updated_post, _| {
            assert_eq!(updated_post.categories, Some(updated_value.clone()));
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
            assert_eq!(updated_post.tags, Some(updated_value.clone()));
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn update_status_to_future() {
    test_update_post(
        &PostUpdateParams {
            status: Some(PostStatus::Future),
            // Publish date has to be in the future
            date: Some(WpDateString("2026-09-09T12:00:00".to_string())),
            ..Default::default()
        },
        |updated_post, updated_post_from_wp_cli| {
            assert_eq!(updated_post.status, PostStatus::Future);
            assert_eq!(
                updated_post_from_wp_cli.post_status,
                PostStatus::Future.to_string()
            );
        },
    )
    .await;
}

// See `update_status_to_future` test case for `PostStatus::Future`
// Note: `PostStatus::Trash` is not tested here because WordPress doesn't allow setting
// a post's status to "trash" via the update endpoint. Use the trash endpoint instead
// (tested in `trash_post` test).
generate_update_post_status_test!(Draft);
generate_update_post_status_test!(Pending);
generate_update_post_status_test!(Private);
generate_update_post_status_test!(Publish);

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
    F: Fn(AnyPostWithEditContext, WpCliPost),
{
    let created_post = api_client()
        .posts()
        .create(&PostEndpointType::Posts, params)
        .await
        .assert_response()
        .data;
    let created_post_from_wp_cli = Backend::post(&created_post.id).await;
    assert(created_post, created_post_from_wp_cli);
    RestoreServer::db().await;
}

async fn test_update_post<F>(params: &PostUpdateParams, assert: F)
where
    F: Fn(AnyPostWithEditContext, WpCliPost),
{
    let updated_post = api_client()
        .posts()
        .update(&PostEndpointType::Posts, &FIRST_POST_ID, params)
        .await
        .assert_response()
        .data;
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

    macro_rules! generate_update_post_status_test {
        ($status:ident) => {
            paste::paste! {
                #[tokio::test]
                #[serial]
                async fn [<update_post_status_to_ $status:lower>]() {
                    test_update_post(
                        &PostUpdateParams {
                            status: Some(PostStatus::$status),
                            ..Default::default()
                        },
                        |updated_post, updated_post_from_wp_cli| {
                            assert_eq!(updated_post.status, PostStatus::$status);
                            assert_eq!(
                                updated_post_from_wp_cli.post_status,
                                PostStatus::$status.to_string()
                            );
                        }
                    ).await;
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
                            assert_eq!(updated_post.format, Some(PostFormat::$format));
                        }
                    ).await;
                }
            }
        };
    }

    pub(super) use generate_update_post_format_test;
    pub(super) use generate_update_post_status_test;
    pub(super) use generate_update_test;
}

// Books custom post type has `genre` and `book-author` custom taxonomies
// registered by the books-plugin. These appear as additional fields on book
// posts, keyed by the taxonomy's rest_base.

#[tokio::test]
#[serial]
async fn create_book_with_custom_taxonomy_terms() {
    let books_endpoint = PostEndpointType::Custom("books".to_string());

    // First, list existing genre terms to get valid IDs
    let books = api_client()
        .posts()
        .list_with_edit_context(&books_endpoint, &PostListParams::default())
        .await
        .assert_response()
        .data;
    let genre_ids: Vec<TermId> = books
        .iter()
        .filter_map(|b| {
            b.additional_fields
                .as_ref()
                .map(|af| af.term_ids_for_key("genre"))
        })
        .find(|ids| !ids.is_empty())
        .expect("Expected at least one book with genres");

    let additional = WpAdditionalFields::from_term_id_map(HashMap::from([(
        "genre".to_string(),
        genre_ids.clone(),
    )]));
    let params = PostCreateParams {
        title: Some("Integration Test Book".to_string()),
        status: Some(PostStatus::Publish),
        additional_fields: Some(additional),
        ..Default::default()
    };
    let created_book = api_client()
        .posts()
        .create(&books_endpoint, &params)
        .await
        .assert_response()
        .data;
    let created_genres = created_book
        .additional_fields
        .as_ref()
        .unwrap()
        .term_ids_for_key("genre");
    assert_eq!(created_genres, genre_ids);

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn update_book_custom_taxonomy_terms() {
    let books_endpoint = PostEndpointType::Custom("books".to_string());

    // List books and collect two different genre IDs
    let books = api_client()
        .posts()
        .list_with_edit_context(&books_endpoint, &PostListParams::default())
        .await
        .assert_response()
        .data;
    let mut all_genre_ids: Vec<TermId> = books
        .iter()
        .filter_map(|b| {
            b.additional_fields
                .as_ref()
                .map(|af| af.term_ids_for_key("genre"))
        })
        .flatten()
        .collect();
    all_genre_ids.sort_by_key(|t| t.0);
    all_genre_ids.dedup();
    assert!(
        all_genre_ids.len() >= 2,
        "Need at least 2 distinct genre IDs for this test"
    );

    let book_id = books[0].id;
    let new_genres = vec![all_genre_ids[0], all_genre_ids[1]];

    let additional = WpAdditionalFields::from_term_id_map(HashMap::from([(
        "genre".to_string(),
        new_genres.clone(),
    )]));
    let params = PostUpdateParams {
        additional_fields: Some(additional),
        ..Default::default()
    };
    let updated_book = api_client()
        .posts()
        .update(&books_endpoint, &book_id, &params)
        .await
        .assert_response()
        .data;
    let mut updated_genres = updated_book
        .additional_fields
        .as_ref()
        .unwrap()
        .term_ids_for_key("genre");
    updated_genres.sort_by_key(|t| t.0);
    assert_eq!(updated_genres, new_genres);

    RestoreServer::db().await;
}
