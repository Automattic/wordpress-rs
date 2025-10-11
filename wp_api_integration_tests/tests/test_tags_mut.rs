use macro_helper::generate_update_test;
use wp_api::{
    request::endpoint::terms_endpoint::TermEndpointType,
    terms::{AnyTermWithEditContext, TermCreateParams, TermUpdateParams},
};
use wp_api_integration_tests::prelude::*;
use wp_cli::WpCliTag;

#[tokio::test]
#[serial]
async fn create_tag_with_just_name() {
    test_create_tag(
        &TermCreateParams {
            name: "foo".to_string(),
            description: None,
            slug: None,
            parent: None,
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
        &TermCreateParams {
            name: "foo".to_string(),
            description: Some("bar".to_string()),
            slug: None,
            parent: None,
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
        &TermCreateParams {
            name: "foo".to_string(),
            description: None,
            slug: Some("bar".to_string()),
            parent: None,
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
        &TermCreateParams {
            name: "foo".to_string(),
            description: Some("bar".to_string()),
            slug: Some("quox".to_string()),
            parent: None,
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
    let tag_delete_response = api_client()
        .terms()
        .delete(&TermEndpointType::Tags, &TAG_ID_100)
        .await;
    assert!(tag_delete_response.is_ok(), "{tag_delete_response:#?}");
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

generate_update_test!(
    update_description,
    description,
    "new_description".to_string(),
    |updated_tag, updated_tag_from_wp_cli| {
        assert_eq!(updated_tag.description, "new_description");
        assert_eq!(updated_tag_from_wp_cli.description, "new_description");
    }
);

generate_update_test!(
    update_name,
    name,
    "new_name".to_string(),
    |updated_tag, updated_tag_from_wp_cli| {
        assert_eq!(updated_tag.name, "new_name");
        assert_eq!(updated_tag_from_wp_cli.name, "new_name");
    }
);

generate_update_test!(
    update_slug,
    slug,
    "new_slug".to_string(),
    |updated_tag, updated_tag_from_wp_cli| {
        assert_eq!(updated_tag.slug, "new_slug");
        assert_eq!(updated_tag_from_wp_cli.slug, "new_slug");
    }
);

async fn test_create_tag<F>(params: &TermCreateParams, assert: F)
where
    F: Fn(AnyTermWithEditContext, WpCliTag),
{
    let created_tag = api_client()
        .terms()
        .create(&TermEndpointType::Tags, params)
        .await
        .assert_response()
        .data;
    let created_tag_from_wp_cli = Backend::tag(&created_tag.id).await;
    assert(created_tag, created_tag_from_wp_cli);
    RestoreServer::db().await;
}

async fn test_update_tag<F>(params: &TermUpdateParams, assert: F)
where
    F: Fn(AnyTermWithEditContext, WpCliTag),
{
    let updated_tag = api_client()
        .terms()
        .update(&TermEndpointType::Tags, &TAG_ID_100, params)
        .await
        .assert_response()
        .data;
    let updated_tag_from_wp_cli = Backend::tag(&TAG_ID_100).await;
    assert(updated_tag, updated_tag_from_wp_cli);
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
                    test_update_tag(
                        &TermUpdateParams {
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
