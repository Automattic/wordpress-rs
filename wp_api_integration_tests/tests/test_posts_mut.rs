use serial_test::serial;
use wp_api::posts::{PostCreateParams, PostWithEditContext};
use wp_api_integration_tests::{
    api_client,
    backend::{Backend, RestoreServer},
    AssertResponse, FIRST_POST_ID,
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

async fn test_create_post<F>(params: &PostCreateParams, assert: F)
where
    F: Fn(PostWithEditContext, WpCliPost),
{
    let created_post = api_client().posts().create(&params).await.assert_response();
    let created_post_from_wp_cli = Backend::post(&created_post.id).await;
    assert(created_post, created_post_from_wp_cli);
    RestoreServer::db().await;
}
