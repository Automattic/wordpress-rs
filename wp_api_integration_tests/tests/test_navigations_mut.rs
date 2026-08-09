use macro_helper::{generate_update_navigation_status_test, generate_update_test};
use wp_api::date::WpDateString;
use wp_api::navigations::{
    NavigationCreateParams, NavigationId, NavigationStatus, NavigationUpdateParams,
    NavigationWithEditContext,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[serial]
async fn create_navigation_with_just_title() {
    test_create_navigation(
        &NavigationCreateParams {
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
        &NavigationCreateParams {
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
        &NavigationCreateParams {
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
        .navigations()
        .delete(&NavigationId(TestCredentials::instance().navigation_id))
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
        .navigations()
        .trash(&NavigationId(TestCredentials::instance().navigation_id))
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
    WpDateString("2024-09-09T12:00:00".to_string()),
    |updated_navigation| {
        assert_eq!(updated_navigation.date.0, "2024-09-09T12:00:00");
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
        assert_eq!(
            updated_navigation.password,
            Some("new_password".to_string())
        );
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
        &NavigationUpdateParams {
            status: Some(NavigationStatus::Future),
            date: Some(WpDateString("2026-09-09T12:00:00".to_string())),
            ..Default::default()
        },
        |updated_navigation| {
            assert_eq!(updated_navigation.status, NavigationStatus::Future);
        },
    )
    .await;
}

generate_update_navigation_status_test!(Draft);
generate_update_navigation_status_test!(Pending);
generate_update_navigation_status_test!(Private);
generate_update_navigation_status_test!(Publish);

async fn test_create_navigation<F>(params: &NavigationCreateParams, assert: F)
where
    F: Fn(NavigationWithEditContext),
{
    let created_navigation = api_client()
        .navigations()
        .create(params)
        .await
        .assert_response()
        .data;
    assert(created_navigation);
    RestoreServer::db().await;
}

async fn test_update_navigation<F>(params: &NavigationUpdateParams, assert: F)
where
    F: Fn(NavigationWithEditContext),
{
    let updated_navigation = api_client()
        .navigations()
        .update(
            &NavigationId(TestCredentials::instance().navigation_id),
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
                        &NavigationUpdateParams {
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
                        &NavigationUpdateParams {
                            status: Some(NavigationStatus::$status),
                            ..Default::default()
                        },
                        |updated_navigation| {
                            assert_eq!(updated_navigation.status, NavigationStatus::$status);
                        }
                    ).await;
                }
            }
        };
    }

    pub(super) use generate_update_navigation_status_test;
    pub(super) use generate_update_test;
}
