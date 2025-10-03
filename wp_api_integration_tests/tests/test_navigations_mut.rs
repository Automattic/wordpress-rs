use macro_helper::{generate_update_navigation_status_test, generate_update_test};
use wp_api::posts::{AnyPostWithEditContext, PostCreateParams, PostStatus, PostUpdateParams};
use wp_api::request::endpoint::posts_endpoint::PostEndpointType;
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[serial]
async fn create_navigation_with_just_title() {
    test_create_navigation(
        &PostCreateParams {
            title: Some("foo".to_string()),
            ..Default::default()
        },
        |created_navigation| {
            assert_eq!(created_navigation.title.raw, Some("foo".to_string()));
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_navigation_with_just_content() {
    test_create_navigation(
        &PostCreateParams {
            content: Some("foo".to_string()),
            ..Default::default()
        },
        |created_navigation| {
            assert_eq!(created_navigation.content.raw, Some("foo".to_string()));
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_navigation_with_title_and_content() {
    test_create_navigation(
        &PostCreateParams {
            title: Some("foo".to_string()),
            content: Some("bar".to_string()),
            ..Default::default()
        },
        |created_navigation| {
            assert_eq!(created_navigation.title.raw, Some("foo".to_string()));
            assert_eq!(created_navigation.content.raw, Some("bar".to_string()));
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn delete_navigation() {
    let navigation_delete_response = api_client()
        .posts()
        .delete(
            &PostEndpointType::Navigation,
            &PostId(TestCredentials::instance().navigation_id),
        )
        .await;
    assert!(
        navigation_delete_response.is_ok(),
        "{navigation_delete_response:#?}"
    );
    assert!(navigation_delete_response.unwrap().data.deleted);

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn trash_navigation() {
    let navigation_trash_response = api_client()
        .posts()
        .trash(
            &PostEndpointType::Navigation,
            &PostId(TestCredentials::instance().navigation_id),
        )
        .await;
    assert!(
        navigation_trash_response.is_ok(),
        "{navigation_trash_response:#?}"
    );

    RestoreServer::db().await;
}

generate_update_test!(
    update_date,
    date,
    "2024-09-09T12:00:00".to_string(),
    |updated_navigation| {
        assert_eq!(updated_navigation.date, "2024-09-09T12:00:00");
    }
);

generate_update_test!(
    update_date_gmt,
    date_gmt,
    unwrapped_wp_gmt_date_time("2024-09-09T12:00:00+0000"),
    |updated_navigation| {
        assert_eq!(
            updated_navigation.date_gmt,
            unwrapped_wp_gmt_date_time("2024-09-09T12:00:00+0000")
        );
    }
);

generate_update_test!(
    update_slug,
    slug,
    "new_slug".to_string(),
    |updated_navigation| {
        assert_eq!(updated_navigation.slug, "new_slug");
    }
);

generate_update_test!(
    update_password,
    password,
    "new_password".to_string(),
    |updated_navigation| {
        assert_eq!(updated_navigation.password, "new_password");
    }
);

generate_update_test!(
    update_title,
    title,
    "new_title".to_string(),
    |updated_navigation| {
        assert_eq!(updated_navigation.title.raw, Some("new_title".to_string()));
    }
);

generate_update_test!(
    update_content,
    content,
    "new_content".to_string(),
    |updated_navigation| {
        assert_eq!(
            updated_navigation.content.raw,
            Some("new_content".to_string())
        );
    }
);

#[tokio::test]
#[serial]
async fn update_status_to_future() {
    test_update_navigation(
        &PostUpdateParams {
            status: Some(PostStatus::Future),
            date: Some("2026-09-09T12:00:00".to_string()),
            ..Default::default()
        },
        |updated_navigation| {
            assert_eq!(updated_navigation.status, PostStatus::Future);
        },
    )
    .await;
}

generate_update_navigation_status_test!(Draft);
generate_update_navigation_status_test!(Pending);
generate_update_navigation_status_test!(Private);
generate_update_navigation_status_test!(Publish);

async fn test_create_navigation<F>(params: &PostCreateParams, assert: F)
where
    F: Fn(AnyPostWithEditContext),
{
    let created_navigation = api_client()
        .posts()
        .create(&PostEndpointType::Navigation, params)
        .await
        .assert_response()
        .data;
    assert(created_navigation);
    RestoreServer::db().await;
}

async fn test_update_navigation<F>(params: &PostUpdateParams, assert: F)
where
    F: Fn(AnyPostWithEditContext),
{
    let updated_navigation = api_client()
        .posts()
        .update(
            &PostEndpointType::Navigation,
            &PostId(TestCredentials::instance().navigation_id),
            params,
        )
        .await
        .assert_response()
        .data;
    assert(updated_navigation);
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
                    test_update_navigation(
                        &PostUpdateParams {
                            $field: Some(updated_value),
                            ..Default::default()
                        }, $assertion)
                    .await;
                }
            }
        };
    }

    macro_rules! generate_update_navigation_status_test {
        ($status:ident) => {
            paste::paste! {
                #[tokio::test]
                #[serial]
                async fn [<update_navigation_status_to_ $status:lower>]() {
                    test_update_navigation(
                        &PostUpdateParams {
                            status: Some(PostStatus::$status),
                            ..Default::default()
                        },
                        |updated_navigation| {
                            assert_eq!(updated_navigation.status, PostStatus::$status);
                        }
                    ).await;
                }
            }
        };
    }

    pub(super) use generate_update_navigation_status_test;
    pub(super) use generate_update_test;
}
