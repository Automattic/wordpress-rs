use macro_helper::{generate_update_block_status_test, generate_update_test};
use wp_api::blocks::{
    BlockCreateParams, BlockId, BlockStatus, BlockUpdateParams, BlockWithEditContext,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[serial]
async fn create_block_with_just_title() {
    test_create_block(
        &BlockCreateParams {
            title: Some("foo".to_string()),
            ..Default::default()
        },
        |created_block| {
            assert_eq!(created_block.title.raw, Some("foo".to_string()));
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_block_with_just_content() {
    test_create_block(
        &BlockCreateParams {
            content: Some("foo".to_string()),
            ..Default::default()
        },
        |created_block| {
            assert_eq!(created_block.content.raw, Some("foo".to_string()));
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_block_with_title_and_content() {
    test_create_block(
        &BlockCreateParams {
            title: Some("foo".to_string()),
            content: Some("bar".to_string()),
            ..Default::default()
        },
        |created_block| {
            assert_eq!(created_block.title.raw, Some("foo".to_string()));
            assert_eq!(created_block.content.raw, Some("bar".to_string()));
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn delete_block() {
    let block_delete_response = api_client()
        .blocks()
        .delete(&BlockId(TestCredentials::instance().block_id))
        .await;
    assert!(block_delete_response.is_ok(), "{block_delete_response:#?}");
    assert!(block_delete_response.unwrap().data.deleted);

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn trash_block() {
    let block_trash_response = api_client()
        .blocks()
        .trash(&BlockId(TestCredentials::instance().block_id))
        .await;
    assert!(block_trash_response.is_ok(), "{block_trash_response:#?}");

    RestoreServer::db().await;
}

generate_update_test!(
    update_date,
    date,
    unwrapped_wp_gmt_date_time("2024-09-09T12:00:00+0000"),
    |updated_block| {
        assert_eq!(updated_block.date.0, "2024-09-09T12:00:00");
    }
);

generate_update_test!(
    update_date_gmt,
    date_gmt,
    unwrapped_wp_gmt_date_time("2024-09-09T12:00:00+0000"),
    |updated_block| {
        assert_eq!(
            updated_block.date_gmt,
            unwrapped_wp_gmt_date_time("2024-09-09T12:00:00+0000")
        );
    }
);

generate_update_test!(update_slug, slug, "new_slug".to_string(), |updated_block| {
    assert_eq!(updated_block.slug, "new_slug");
});

generate_update_test!(
    update_password,
    password,
    "new_password".to_string(),
    |updated_block| {
        assert_eq!(updated_block.password, Some("new_password".to_string()));
    }
);

generate_update_test!(
    update_title,
    title,
    "new_title".to_string(),
    |updated_block| {
        assert_eq!(updated_block.title.raw, Some("new_title".to_string()));
    }
);

generate_update_test!(
    update_content,
    content,
    "new_content".to_string(),
    |updated_block| {
        assert_eq!(updated_block.content.raw, Some("new_content".to_string()));
    }
);

#[tokio::test]
#[serial]
async fn update_status_to_future() {
    test_update_block(
        &BlockUpdateParams {
            status: Some(BlockStatus::Future),
            date: Some(unwrapped_wp_gmt_date_time("2026-09-09T12:00:00+0000")),
            ..Default::default()
        },
        |updated_block| {
            assert_eq!(updated_block.status, BlockStatus::Future);
        },
    )
    .await;
}

generate_update_block_status_test!(Draft);
generate_update_block_status_test!(Pending);
generate_update_block_status_test!(Private);
generate_update_block_status_test!(Publish);

async fn test_create_block<F>(params: &BlockCreateParams, assert: F)
where
    F: Fn(BlockWithEditContext),
{
    let created_block = api_client()
        .blocks()
        .create(params)
        .await
        .assert_response()
        .data;
    assert(created_block);
    RestoreServer::db().await;
}

async fn test_update_block<F>(params: &BlockUpdateParams, assert: F)
where
    F: Fn(BlockWithEditContext),
{
    let updated_block = api_client()
        .blocks()
        .update(&BlockId(TestCredentials::instance().block_id), params)
        .await
        .assert_response()
        .data;
    assert(updated_block);
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
                    test_update_block(
                        &BlockUpdateParams {
                            $field: Some(updated_value),
                            ..Default::default()
                        }, $assertion)
                    .await;
                }
            }
        };
    }

    macro_rules! generate_update_block_status_test {
        ($status:ident) => {
            paste::paste! {
                #[tokio::test]
                #[serial]
                async fn [<update_block_status_to_ $status:lower>]() {
                    test_update_block(
                        &BlockUpdateParams {
                            status: Some(BlockStatus::$status),
                            ..Default::default()
                        },
                        |updated_block| {
                            assert_eq!(updated_block.status, BlockStatus::$status);
                        }
                    ).await;
                }
            }
        };
    }

    pub(super) use generate_update_block_status_test;
    pub(super) use generate_update_test;
}
