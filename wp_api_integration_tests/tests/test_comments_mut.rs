use macro_helper::generate_update_test;
use wp_api::comments::{
    CommentCreateParams, CommentCreateParamsBuilder, CommentDeleteParams, CommentStatus,
    CommentUpdateParams, CommentWithEditContext, CommentWithViewContext,
};
use wp_api_integration_tests::prelude::*;
use wp_cli::WpCliComment;

#[tokio::test]
#[serial]
async fn create_comment_with_just_content() {
    test_create_comment(
        &CommentCreateParams::new(FIRST_POST_ID, "foo".to_string()),
        |created_comment, comment_from_wp_cli| {
            assert!(created_comment.content.rendered.contains("foo"));
            assert_eq!(comment_from_wp_cli.content, "foo");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_comment_with_content_and_status() {
    test_create_comment(
        &CommentCreateParamsBuilder::new(FIRST_POST_ID, "foo".to_string())
            .status(Some(CommentStatus::Hold))
            .build(),
        |created_comment, comment_from_wp_cli| {
            assert!(created_comment.content.rendered.contains("foo"));
            assert_eq!(created_comment.status, CommentStatus::Hold);
            assert_eq!(comment_from_wp_cli.content, "foo");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_comment_as_subscriber() {
    // Core returns a view-context response to users without `moderate_comments`,
    // so the create response must parse for a subscriber as well.
    let created_comment = api_client_as_subscriber()
        .comments()
        .create(&CommentCreateParams::new(FIRST_POST_ID, "foo".to_string()))
        .await
        .assert_response()
        .data;
    let created_comment_from_wp_cli = Backend::comment(&created_comment.id).await;
    assert_eq!(created_comment_from_wp_cli.content, "foo");
    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn delete_comment() {
    // Delete the comment using the API and ensure it's successful
    let comment_delete_response = api_client()
        .comments()
        .delete(&FIRST_COMMENT_ID, &CommentDeleteParams::default())
        .await;
    assert!(
        comment_delete_response.is_ok(),
        "{comment_delete_response:#?}"
    );
    assert!(comment_delete_response.unwrap().data.deleted);

    // Assert that the comment was deleted
    assert!(
        !Backend::comments(None)
            .await
            .into_iter()
            .any(|c| c.comment_id == FIRST_COMMENT_ID.0),
        "Comment wasn't deleted"
    );

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn trash_comment() {
    // Trash the comment using the API and ensure it's successful
    let comment_trash_response = api_client()
        .comments()
        .trash(&FIRST_COMMENT_ID, &CommentDeleteParams::default())
        .await;
    assert!(
        comment_trash_response.is_ok(),
        "{comment_trash_response:#?}"
    );

    // Assert that the comment was trashed
    let trashed_comment = Backend::comments(Some("trash"))
        .await
        .into_iter()
        .find(|c| c.comment_id == FIRST_COMMENT_ID.0);
    assert!(trashed_comment.is_some(), "Can't find the trashed comment");

    RestoreServer::db().await;
}

generate_update_test!(
    update_author,
    author,
    SECOND_USER_ID,
    |updated_comment, updated_comment_from_wp_cli| {
        assert_eq!(updated_comment.author, SECOND_USER_ID);
        assert_eq!(updated_comment_from_wp_cli.author, SECOND_USER_ID.0);
    }
);

generate_update_test!(
    update_author_email,
    author_email,
    "foo@example.com".to_string(),
    |updated_comment, updated_comment_from_wp_cli| {
        assert_eq!(updated_comment.author_email, "foo@example.com");
        assert_eq!(updated_comment_from_wp_cli.author_email, "foo@example.com");
    }
);

generate_update_test!(
    update_author_ip,
    author_ip,
    "127.0.0.1".to_string(),
    |updated_comment, updated_comment_from_wp_cli| {
        assert_eq!(updated_comment.author_ip, "127.0.0.1");
        assert_eq!(updated_comment_from_wp_cli.author_ip, "127.0.0.1");
    }
);

generate_update_test!(
    update_author_name,
    author_name,
    "foo".to_string(),
    |updated_comment, updated_comment_from_wp_cli| {
        assert_eq!(updated_comment.author_name, "foo");
        assert_eq!(updated_comment_from_wp_cli.author_name, "foo");
    }
);

generate_update_test!(
    update_author_url,
    author_url,
    "https://example.com".to_string(),
    |updated_comment, updated_comment_from_wp_cli| {
        assert_eq!(updated_comment.author_url, "https://example.com");
        assert_eq!(
            updated_comment_from_wp_cli.author_url,
            "https://example.com"
        );
    }
);

generate_update_test!(
    update_author_user_agent,
    author_user_agent,
    "foo".to_string(),
    |updated_comment, updated_comment_from_wp_cli| {
        assert_eq!(updated_comment.author_user_agent, "foo");
        assert_eq!(updated_comment_from_wp_cli.author_user_agent, "foo");
    }
);

generate_update_test!(
    update_content,
    content,
    "foo".to_string(),
    |updated_comment, updated_comment_from_wp_cli| {
        assert_eq!(updated_comment.content.raw, "foo");
        assert_eq!(updated_comment_from_wp_cli.content, "foo");
    }
);

generate_update_test!(
    update_date,
    date,
    "2024-09-09T12:00:00".to_string(),
    |updated_comment, updated_comment_from_wp_cli| {
        assert_eq!(updated_comment.date, "2024-09-09T12:00:00");
        assert_eq!(updated_comment_from_wp_cli.date, "2024-09-09 12:00:00");
    }
);

generate_update_test!(
    update_date_gmt,
    date_gmt,
    unwrapped_wp_gmt_date_time("2024-09-09T12:00:00+0000"),
    |updated_comment, updated_comment_from_wp_cli| {
        assert_eq!(
            updated_comment.date_gmt,
            unwrapped_wp_gmt_date_time("2024-09-09T12:00:00+0000")
        );
        assert_eq!(updated_comment_from_wp_cli.date_gmt, "2024-09-09 12:00:00");
    }
);

generate_update_test!(
    update_parent,
    parent,
    SECOND_COMMENT_ID,
    |updated_comment, updated_comment_from_wp_cli| {
        assert_eq!(updated_comment.parent, SECOND_COMMENT_ID);
        assert_eq!(updated_comment_from_wp_cli.parent, SECOND_COMMENT_ID.0);
    }
);

generate_update_test!(
    update_post,
    post,
    POST_ID_555,
    |updated_comment, updated_comment_from_wp_cli| {
        assert_eq!(updated_comment.post, POST_ID_555);
        assert_eq!(updated_comment_from_wp_cli.post, POST_ID_555.0);
    }
);

generate_update_test!(
    update_status,
    status,
    CommentStatus::Hold,
    |updated_comment, _| {
        assert_eq!(updated_comment.status, CommentStatus::Hold);
    }
);

async fn test_create_comment<F>(params: &CommentCreateParams, assert: F)
where
    F: Fn(CommentWithViewContext, WpCliComment),
{
    let created_comment = api_client()
        .comments()
        .create(params)
        .await
        .assert_response()
        .data;
    let created_comment_from_wp_cli = Backend::comment(&created_comment.id).await;
    assert(created_comment, created_comment_from_wp_cli);
    RestoreServer::db().await;
}

async fn test_update_comment<F>(params: &CommentUpdateParams, assert: F)
where
    F: Fn(CommentWithEditContext, WpCliComment),
{
    let updated_comment = api_client()
        .comments()
        .update(&FIRST_COMMENT_ID, params)
        .await
        .assert_response()
        .data;
    let updated_comment_from_wp_cli = Backend::comment(&FIRST_COMMENT_ID).await;
    assert(updated_comment, updated_comment_from_wp_cli);
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
                    test_update_comment(
                        &CommentUpdateParams {
                            $field: Some(updated_value),
                            ..Default::default()
                        }, $assertion)
                    .await;
                }
            }
        };
    }

    pub(super) use generate_update_test;
}
