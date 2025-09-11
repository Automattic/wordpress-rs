use macro_helper::{generate_update_page_status_test, generate_update_test};
use wp_api::pages::{
    PageCommentStatus, PageCreateParams, PageFootnote, PageMeta, PagePingStatus, PageStatus,
    PageUpdateParams, PageWithEditContext,
};
use wp_api_integration_tests::{PAGE_TEMPLATE_WITH_SIDEBAR, prelude::*};
use wp_cli::WpCliPage;

#[tokio::test]
#[serial]
async fn create_page_with_just_title() {
    test_create_page(
        &PageCreateParams {
            title: Some("foo".to_string()),
            ..Default::default()
        },
        |created_page, page_from_wp_cli| {
            assert_eq!(created_page.title.raw, Some("foo".to_string()));
            assert_eq!(page_from_wp_cli.title, "foo");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_page_with_title_and_meta() {
    test_create_page(
        &PageCreateParams {
            title: Some("foo".to_string()),
            meta: Some(PageMeta {
                footnotes: vec![PageFootnote {
                    id: "bar".to_string(),
                    content: "baz".to_string(),
                }],
            }),
            ..Default::default()
        },
        |created_page, page_from_wp_cli| {
            let footnote = created_page.meta.footnotes.first().unwrap();
            assert_eq!(created_page.title.raw, Some("foo".to_string()));
            assert_eq!(page_from_wp_cli.title, "foo");
            assert_eq!(footnote.id, "bar");
            assert_eq!(footnote.content, "baz");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_page_with_just_content() {
    test_create_page(
        &PageCreateParams {
            content: Some("foo".to_string()),
            ..Default::default()
        },
        |created_page, page_from_wp_cli| {
            assert_eq!(created_page.content.raw, Some("foo".to_string()));
            assert_eq!(page_from_wp_cli.content, "foo");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_page_with_just_excerpt() {
    test_create_page(
        &PageCreateParams {
            excerpt: Some("foo".to_string()),
            ..Default::default()
        },
        |created_page, page_from_wp_cli| {
            assert_eq!(created_page.excerpt.raw, Some("foo".to_string()));
            assert_eq!(page_from_wp_cli.excerpt, "foo");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_page_with_title_content_and_excerpt() {
    test_create_page(
        &PageCreateParams {
            title: Some("foo".to_string()),
            content: Some("bar".to_string()),
            excerpt: Some("baz".to_string()),
            ..Default::default()
        },
        |created_page, page_from_wp_cli| {
            assert_eq!(created_page.title.raw, Some("foo".to_string()));
            assert_eq!(page_from_wp_cli.title, "foo");
            assert_eq!(created_page.content.raw, Some("bar".to_string()));
            assert_eq!(page_from_wp_cli.content, "bar");
            assert_eq!(created_page.excerpt.raw, Some("baz".to_string()));
            assert_eq!(page_from_wp_cli.excerpt, "baz");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn delete_page() {
    // Delete the page using the API and ensure it's successful
    let page_delete_response = api_client()
        .pages()
        .delete(&PageId(TestCredentials::instance().first_page_id))
        .await;
    assert!(page_delete_response.is_ok(), "{page_delete_response:#?}");
    assert!(page_delete_response.unwrap().data.deleted);

    // Assert that the page was deleted
    assert!(
        !Backend::pages(None)
            .await
            .into_iter()
            .any(|p| p.id == PageId(TestCredentials::instance().first_page_id).0),
        "Page wasn't deleted"
    );

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn trash_page() {
    // Trash the page using the API and ensure it's successful
    let page_trash_response = api_client()
        .pages()
        .trash(&PageId(TestCredentials::instance().first_page_id))
        .await;
    assert!(page_trash_response.is_ok(), "{page_trash_response:#?}");

    // Assert that the page was trashed
    let trashed_page = Backend::pages(Some("trash"))
        .await
        .into_iter()
        .find(|p| p.id == PageId(TestCredentials::instance().first_page_id).0);
    assert!(trashed_page.is_some(), "Can't find the trashed page");
    assert_eq!(
        trashed_page.unwrap().post_status,
        "trash",
        "Page wasn't trashed"
    );

    RestoreServer::db().await;
}

generate_update_test!(
    update_date,
    date,
    "2024-09-09T12:00:00".to_string(),
    |updated_page, updated_page_from_wp_cli| {
        assert_eq!(updated_page.date, "2024-09-09T12:00:00");
        assert_eq!(updated_page_from_wp_cli.date, "2024-09-09 12:00:00");
    }
);

generate_update_test!(
    update_date_gmt,
    date_gmt,
    unwrapped_wp_gmt_date_time("2024-09-09T12:00:00+0000"),
    |updated_page, updated_page_from_wp_cli| {
        assert_eq!(
            updated_page.date_gmt,
            unwrapped_wp_gmt_date_time("2024-09-09T12:00:00+0000")
        );
        assert_eq!(updated_page_from_wp_cli.date_gmt, "2024-09-09 12:00:00");
    }
);

generate_update_test!(
    update_slug,
    slug,
    "new_slug".to_string(),
    |updated_page, updated_page_from_wp_cli| {
        assert_eq!(updated_page.slug, "new_slug");
        assert_eq!(updated_page_from_wp_cli.slug, "new_slug");
    }
);

generate_update_test!(
    update_password,
    password,
    "new_password".to_string(),
    |updated_page, updated_page_from_wp_cli| {
        assert_eq!(updated_page.password, "new_password");
        assert_eq!(updated_page_from_wp_cli.password, "new_password");
    }
);

generate_update_test!(
    update_title,
    title,
    "new_title".to_string(),
    |updated_page, updated_page_from_wp_cli| {
        assert_eq!(updated_page.title.raw, Some("new_title".to_string()));
        assert_eq!(updated_page_from_wp_cli.title, "new_title");
    }
);

generate_update_test!(
    update_content,
    content,
    "new_content".to_string(),
    |updated_page, updated_page_from_wp_cli| {
        assert_eq!(updated_page.content.raw, Some("new_content".to_string()));
        assert_eq!(updated_page_from_wp_cli.content, "new_content");
    }
);

generate_update_test!(
    update_author,
    author,
    SECOND_USER_ID,
    |updated_page, updated_page_from_wp_cli| {
        assert_eq!(updated_page.author, SECOND_USER_ID);
        assert_eq!(updated_page_from_wp_cli.author, SECOND_USER_ID.0);
    }
);

generate_update_test!(
    update_excerpt,
    excerpt,
    "new_excerpt".to_string(),
    |updated_page, updated_page_from_wp_cli| {
        assert_eq!(updated_page.excerpt.raw, Some("new_excerpt".to_string()));
        assert_eq!(updated_page_from_wp_cli.excerpt, "new_excerpt");
    }
);

generate_update_test!(
    update_featured_media,
    featured_media,
    MEDIA_ID_611,
    |updated_page, _| {
        assert_eq!(updated_page.featured_media, MEDIA_ID_611);
    }
);

generate_update_test!(
    update_comment_status_to_open,
    comment_status,
    PageCommentStatus::Open,
    |updated_page, updated_page_from_wp_cli| {
        assert_eq!(updated_page.comment_status, PageCommentStatus::Open);
        assert_eq!(
            updated_page_from_wp_cli.comment_status,
            PageCommentStatus::Open.to_string()
        );
    }
);

generate_update_test!(
    update_comment_status_to_closed,
    comment_status,
    PageCommentStatus::Closed,
    |updated_page, updated_page_from_wp_cli| {
        assert_eq!(updated_page.comment_status, PageCommentStatus::Closed);
        assert_eq!(
            updated_page_from_wp_cli.comment_status,
            PageCommentStatus::Closed.to_string()
        );
    }
);

generate_update_test!(
    update_ping_status_to_open,
    ping_status,
    PagePingStatus::Open,
    |updated_page, updated_page_from_wp_cli| {
        assert_eq!(updated_page.ping_status, PagePingStatus::Open);
        assert_eq!(
            updated_page_from_wp_cli.ping_status,
            PagePingStatus::Open.to_string()
        );
    }
);

generate_update_test!(
    update_ping_status_to_closed,
    ping_status,
    PagePingStatus::Closed,
    |updated_page, updated_page_from_wp_cli| {
        assert_eq!(updated_page.ping_status, PagePingStatus::Closed);
        assert_eq!(
            updated_page_from_wp_cli.ping_status,
            PagePingStatus::Closed.to_string()
        );
    }
);

generate_update_test!(
    update_parent,
    parent,
    PageId(TestCredentials::instance().password_protected_page_id),
    |updated_page, updated_page_from_wp_cli| {
        assert_eq!(
            updated_page.parent,
            PageId(TestCredentials::instance().password_protected_page_id)
        );
        assert_eq!(
            updated_page_from_wp_cli.parent,
            PageId(TestCredentials::instance().password_protected_page_id).0
        );
    }
);

generate_update_test!(
    update_menu_order,
    menu_order,
    5u32,
    |updated_page, updated_page_from_wp_cli| {
        assert_eq!(updated_page.menu_order, 5);
        assert_eq!(updated_page_from_wp_cli.menu_order, 5);
    }
);

generate_update_test!(
    update_template,
    template,
    PAGE_TEMPLATE_WITH_SIDEBAR.to_string(),
    |updated_page, _| {
        assert_eq!(updated_page.template, PAGE_TEMPLATE_WITH_SIDEBAR);
    }
);

generate_update_test!(
    update_meta_to_add_footnote,
    meta,
    PageMeta {
        footnotes: vec![PageFootnote {
            id: "foo".to_string(),
            content: "bar".to_string()
        }]
    },
    |updated_page, _| {
        let footnote = updated_page.meta.footnotes.first().unwrap();
        assert_eq!(footnote.id, "foo");
        assert_eq!(footnote.content, "bar");
    }
);

#[tokio::test]
#[serial]
async fn update_status_to_future() {
    test_update_page(
        &PageUpdateParams {
            status: Some(PageStatus::Future),
            // Publish date has to be in the future
            date: Some("2026-09-09T12:00:00".to_string()),
            ..Default::default()
        },
        |updated_page, updated_page_from_wp_cli| {
            assert_eq!(updated_page.status, PageStatus::Future);
            assert_eq!(
                updated_page_from_wp_cli.post_status,
                PageStatus::Future.to_string()
            );
        },
    )
    .await;
}

// See `update_status_to_future` test case for `PageStatus::Future`
generate_update_page_status_test!(Draft);
generate_update_page_status_test!(Pending);
generate_update_page_status_test!(Private);
generate_update_page_status_test!(Publish);

async fn test_create_page<F>(params: &PageCreateParams, assert: F)
where
    F: Fn(PageWithEditContext, WpCliPage),
{
    let created_page = api_client()
        .pages()
        .create(params)
        .await
        .assert_response()
        .data;
    let created_page_from_wp_cli = Backend::page(&created_page.id).await;
    assert(created_page, created_page_from_wp_cli);
    RestoreServer::db().await;
}

async fn test_update_page<F>(params: &PageUpdateParams, assert: F)
where
    F: Fn(PageWithEditContext, WpCliPage),
{
    let updated_page = api_client()
        .pages()
        .update(&PageId(TestCredentials::instance().first_page_id), params)
        .await
        .assert_response()
        .data;
    let updated_page_from_wp_cli =
        Backend::page(&PageId(TestCredentials::instance().first_page_id)).await;
    assert(updated_page, updated_page_from_wp_cli);
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
                    test_update_page(
                        &PageUpdateParams {
                            $field: Some(updated_value),
                            ..Default::default()
                        }, $assertion)
                    .await;
                }
            }
        };
    }

    macro_rules! generate_update_page_status_test {
        ($status:ident) => {
            paste::paste! {
                #[tokio::test]
                #[serial]
                async fn [<update_page_status_to_ $status:lower>]() {
                    test_update_page(
                        &PageUpdateParams {
                            status: Some(PageStatus::$status),
                            ..Default::default()
                        },
                        |updated_page, updated_page_from_wp_cli| {
                            assert_eq!(updated_page.status, PageStatus::$status);
                            assert_eq!(
                                updated_page_from_wp_cli.post_status,
                                PageStatus::$status.to_string()
                            );
                        }
                    ).await;
                }
            }
        };
    }

    pub(super) use generate_update_page_status_test;
    pub(super) use generate_update_test;
}
