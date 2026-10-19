//! Making working copies from posts and pages the test server returns.
//!
//! The unit tests in `wp_mobile_local_content::remote` cover the conversion
//! rules on hand-built posts. These check them against real REST responses for
//! the status, password and date combinations that matter.

use std::sync::Arc;
use tempfile::TempDir;
use wp_api::{
    date::WpDateString,
    posts::{
        AnyPostWithEditContext, PostCreateParams, PostId, PostRetrieveParams, PostStatus,
        PostUpdateParams,
    },
    prelude::*,
    request::endpoint::posts_endpoint::PostEndpointType,
};
use wp_mobile::service::local_content::LocalContentService;
use wp_mobile_integration_tests::*;
use wp_mobile_local_content::{
    LocalContentError, WpLocalContentStore,
    db_types::{
        working_copy::{PublishDate, StatusSetting},
        working_copy_target::SiteIdentity,
    },
    remote::RemotePostConversionError,
};

fn open_service(dir: &TempDir) -> LocalContentService {
    let path = dir
        .path()
        .join("local_content.db")
        .to_str()
        .expect("Temp path should be UTF-8")
        .to_string();
    let store = WpLocalContentStore::new(path).expect("Opening the store should succeed");
    store
        .perform_migrations()
        .expect("Migrations should succeed");
    LocalContentService::new(Arc::new(store))
}

fn site() -> SiteIdentity {
    SiteIdentity {
        url: Arc::new(
            ParsedUrl::parse(TestCredentials::instance().site_url)
                .expect("Test site URL should parse"),
        ),
        wordpress_com_site_id: None,
    }
}

/// Create a post on the server, then fetch it with the edit context the way
/// an app would before editing it.
async fn create_and_fetch(
    api: &WpApiClient,
    endpoint: &PostEndpointType,
    params: PostCreateParams,
) -> AnyPostWithEditContext {
    let created = api
        .posts()
        .create(endpoint, &params)
        .await
        .expect("Creating the post should succeed")
        .data;
    fetch(api, endpoint, created.id).await
}

/// Save the post once more, as any later edit would. On insert WordPress sets
/// a post's modified date to its date, which the block editor's floating-date
/// rule can't tell apart from a floating date.
async fn save_again(api: &WpApiClient, endpoint: &PostEndpointType, id: PostId) {
    api.posts()
        .update(
            endpoint,
            &id,
            &PostUpdateParams {
                excerpt: Some("Saved again".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("Updating the post should succeed");
}

async fn fetch(
    api: &WpApiClient,
    endpoint: &PostEndpointType,
    id: PostId,
) -> AnyPostWithEditContext {
    api.posts()
        .retrieve_with_edit_context(endpoint, &id, &PostRetrieveParams::default())
        .await
        .expect("Fetching the post should succeed")
        .data
}

#[tokio::test]
#[serial]
async fn test_working_copies_from_server_posts_keep_status_password_and_date() {
    let ctx = create_test_context();
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let service = open_service(&dir);

    let cases = [
        (
            // `wp_insert_post` clears the password of a private post.
            "private, sent with a password",
            PostCreateParams {
                title: Some("Private".to_string()),
                status: Some(PostStatus::Private),
                password: Some("secret".to_string()),
                ..Default::default()
            },
            StatusSetting::Private,
            None,
        ),
        (
            "draft with a password",
            PostCreateParams {
                title: Some("Draft with a password".to_string()),
                status: Some(PostStatus::Draft),
                password: Some("secret".to_string()),
                ..Default::default()
            },
            StatusSetting::Draft {
                password: Some("secret".to_string()),
            },
            Some(PublishDate::Immediately),
        ),
        (
            "draft without a date",
            PostCreateParams {
                title: Some("Floating draft".to_string()),
                status: Some(PostStatus::Draft),
                ..Default::default()
            },
            StatusSetting::Draft { password: None },
            Some(PublishDate::Immediately),
        ),
        (
            "draft with a date, saved again",
            PostCreateParams {
                title: Some("Dated draft".to_string()),
                status: Some(PostStatus::Draft),
                date: Some(WpDateString::new("2030-06-01T10:00:00".to_string())),
                ..Default::default()
            },
            StatusSetting::Draft { password: None },
            None,
        ),
        (
            "scheduled",
            PostCreateParams {
                title: Some("Scheduled".to_string()),
                status: Some(PostStatus::Future),
                date: Some(WpDateString::new("2050-01-01T12:00:00".to_string())),
                ..Default::default()
            },
            StatusSetting::Publish { password: None },
            None,
        ),
    ];

    for (case, params, expected_status, expected_publish_date) in cases {
        let mut post = create_and_fetch(&ctx.api, &PostEndpointType::Posts, params).await;
        if case.ends_with("saved again") {
            save_again(&ctx.api, &PostEndpointType::Posts, post.id).await;
            post = fetch(&ctx.api, &PostEndpointType::Posts, post.id).await;
        }
        let (post_id, date_gmt) = (post.id, post.date_gmt);

        let stored = service
            .read_or_create_post_working_copy_from_remote(site(), post)
            .unwrap_or_else(|e| panic!("{case}: creating the working copy should succeed: {e}"));

        let common = &stored.working_copy.common;
        assert_eq!(
            common.status, expected_status,
            "{case}: status and password"
        );
        // A post whose date doesn't float keeps the server's date.
        let expected_publish_date =
            expected_publish_date.unwrap_or(PublishDate::At { date: date_gmt });
        assert_eq!(
            common.publish_date, expected_publish_date,
            "{case}: publish date"
        );
        assert_eq!(
            stored.target.remote_post.map(|remote| remote.post_id),
            Some(post_id),
            "{case}: the target should point at the remote post"
        );
    }

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn test_working_copy_from_server_page_keeps_its_parent() {
    let ctx = create_test_context();
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let service = open_service(&dir);

    let parent = create_and_fetch(
        &ctx.api,
        &PostEndpointType::Pages,
        PostCreateParams {
            title: Some("Parent".to_string()),
            status: Some(PostStatus::Publish),
            ..Default::default()
        },
    )
    .await;
    let parent_id = parent.id;
    let child = create_and_fetch(
        &ctx.api,
        &PostEndpointType::Pages,
        PostCreateParams {
            title: Some("Child".to_string()),
            status: Some(PostStatus::Publish),
            parent: Some(parent.id),
            ..Default::default()
        },
    )
    .await;

    let parent_page = service
        .read_or_create_page_working_copy_from_remote(site(), parent)
        .expect("Creating the working copy should succeed");
    let child_page = service
        .read_or_create_page_working_copy_from_remote(site(), child)
        .expect("Creating the working copy should succeed");

    assert_eq!(parent_page.working_copy.parent, None);
    assert_eq!(child_page.working_copy.parent, Some(parent_id));

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn test_a_server_post_with_a_working_copy_gets_it_back() {
    let ctx = create_test_context();
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let service = open_service(&dir);
    let post = create_and_fetch(
        &ctx.api,
        &PostEndpointType::Posts,
        PostCreateParams {
            title: Some("On the server".to_string()),
            ..Default::default()
        },
    )
    .await;

    let post_id = post.id;

    let created = service
        .read_or_create_post_working_copy_from_remote(site(), post)
        .expect("Creating the working copy should succeed");
    let mut edited = created.working_copy.clone();
    edited.common.title = "Edited on the device".to_string();
    service
        .save_post_working_copy(created.id.clone(), edited.clone())
        .expect("Saving should succeed");
    let fetched_again = fetch(&ctx.api, &PostEndpointType::Posts, post_id).await;
    let read_again = service
        .read_or_create_post_working_copy_from_remote(site(), fetched_again)
        .expect("Reading the working copy should succeed");

    assert_eq!(read_again.id, created.id);
    assert_eq!(read_again.working_copy, edited);

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn test_trashed_server_post_is_refused() {
    let ctx = create_test_context();
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let service = open_service(&dir);

    let post = create_and_fetch(
        &ctx.api,
        &PostEndpointType::Posts,
        PostCreateParams {
            title: Some("To trash".to_string()),
            ..Default::default()
        },
    )
    .await;
    ctx.api
        .posts()
        .trash(&PostEndpointType::Posts, &post.id)
        .await
        .expect("Trashing the post should succeed");
    let trashed = ctx
        .api
        .posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Posts,
            &PostId(post.id.0),
            &PostRetrieveParams::default(),
        )
        .await
        .expect("Fetching the trashed post should succeed")
        .data;

    let result = service.read_or_create_post_working_copy_from_remote(site(), trashed);

    assert_eq!(
        result,
        Err(LocalContentError::RemotePostConversion {
            error: RemotePostConversionError::UneditableStatus {
                status: "trash".to_string(),
            },
        })
    );

    RestoreServer::db().await;
}
