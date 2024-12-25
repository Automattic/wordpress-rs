use serial_test::serial;
use wp_api::tags::{TagCreateParams, TagWithEditContext};
use wp_api_integration_tests::backend::{Backend, RestoreServer};
use wp_api_integration_tests::{api_client, AssertResponse, TAG_ID_100};
use wp_cli::WpCliTag;

#[tokio::test]
#[serial]
async fn create_tag_with_just_name() {
    test_create_tag(
        &TagCreateParams {
            name: "foo".to_string(),
            description: None,
            slug: None,
        },
        |created_tag, tag_from_wp_cli| {
            assert_eq!(created_tag.name, "foo");
            assert_eq!(tag_from_wp_cli.name, "foo");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_tag_with_name_and_description() {
    test_create_tag(
        &TagCreateParams {
            name: "foo".to_string(),
            description: Some("bar".to_string()),
            slug: None,
        },
        |created_tag, tag_from_wp_cli| {
            assert_eq!(created_tag.name, "foo");
            assert_eq!(created_tag.description, "bar");
            assert_eq!(tag_from_wp_cli.description, "bar");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_tag_with_name_and_slug() {
    test_create_tag(
        &TagCreateParams {
            name: "foo".to_string(),
            description: None,
            slug: Some("bar".to_string()),
        },
        |created_tag, tag_from_wp_cli| {
            assert_eq!(created_tag.name, "foo");
            assert_eq!(created_tag.slug, "bar");
            assert_eq!(tag_from_wp_cli.slug, "bar");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_tag_with_name_description_and_slug() {
    test_create_tag(
        &TagCreateParams {
            name: "foo".to_string(),
            description: Some("bar".to_string()),
            slug: Some("quox".to_string()),
        },
        |created_tag, tag_from_wp_cli| {
            assert_eq!(created_tag.name, "foo");
            assert_eq!(created_tag.description, "bar");
            assert_eq!(tag_from_wp_cli.description, "bar");
            assert_eq!(created_tag.slug, "quox");
            assert_eq!(tag_from_wp_cli.slug, "quox");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn delete_tag() {
    // Delete the tag using the API and ensure it's successful
    let tag_delete_response = api_client().tags().delete(&TAG_ID_100).await;
    assert!(tag_delete_response.is_ok(), "{:#?}", tag_delete_response);
    assert!(tag_delete_response.unwrap().data.deleted);

    // Assert that the tag was deleted
    assert!(
        !Backend::tags()
            .await
            .into_iter()
            .any(|u| u.id == TAG_ID_100.0),
        "Tag wasn't deleted"
    );

    RestoreServer::db().await;
}

async fn test_create_tag<F>(params: &TagCreateParams, assert: F)
where
    F: Fn(TagWithEditContext, WpCliTag),
{
    let created_tag = api_client()
        .tags()
        .create(params)
        .await
        .assert_response()
        .data;
    let created_tag_from_wp_cli = Backend::tag(&created_tag.id).await;
    assert(created_tag, created_tag_from_wp_cli);
    RestoreServer::db().await;
}
