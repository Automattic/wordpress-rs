//! Saving posts from an editor through `PostEditorService`, against the test
//! server.

use serde_json::{Value, json};
use std::{collections::BTreeSet, sync::Arc};
use tempfile::TempDir;
use wp_api::{
    api_client::{IsWpApiClientDelegate, WpApiRequestBuilder},
    middleware::PerformsRequests,
    posts::{PostCreateParams, PostId, PostRetrieveParams, PostStatus, PostUpdateParams},
    prelude::*,
    request::{InnerRequestBuilder, endpoint::posts_endpoint::PostEndpointType},
};
use wp_mobile::service::{
    local_content::LocalContentService,
    post_editor::{EditorRequest, PostEditorService},
};
use wp_mobile_integration_tests::*;
use wp_mobile_local_content::{
    WpLocalContentStore, db_types::working_copy_remote_target::SiteIdentity,
};

fn open_local_content(dir: &TempDir) -> Arc<LocalContentService> {
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
    Arc::new(LocalContentService::new(Arc::new(store)))
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

fn json_body(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("The response should be JSON")
}

fn keys(post: &Value) -> BTreeSet<String> {
    post.as_object()
        .expect("A post is a JSON object")
        .keys()
        .cloned()
        .collect()
}

async fn create_draft(api: &WpApiClient, title: &str) -> PostId {
    api.posts()
        .create(
            &PostEndpointType::Posts,
            &PostCreateParams {
                title: Some(title.to_string()),
                content: Some(
                    "<!-- wp:paragraph --><p>Content</p><!-- /wp:paragraph -->".to_string(),
                ),
                excerpt: Some("An excerpt".to_string()),
                status: Some(PostStatus::Draft),
                ..Default::default()
            },
        )
        .await
        .expect("Creating the post should succeed")
        .data
        .id
}

/// The post's raw JSON with the edit context, as the server returns it.
async fn fetch_raw(api: &WpApiClient, post_id: PostId) -> Value {
    let auth_provider = api.posts().get_delegate().auth_provider.clone();
    let mut url = test_site_api_url_resolver()
        .resolve(
            "wp/v2".to_string(),
            vec!["posts".to_string(), post_id.0.to_string()],
        )
        .inner
        .clone();
    url.query_pairs_mut().append_pair("context", "edit");
    let request = InnerRequestBuilder::new(auth_provider).get(url.into());
    let response = api
        .posts()
        .perform(Arc::new(request), None)
        .await
        .expect("Fetching the post should succeed");
    assert_eq!(response.status_code, 200);
    json_body(&response.body)
}

/// The proposal's "Not verified yet" point: an update response holds the
/// whole post in the edit context, whichever fields the request sent.
#[tokio::test]
#[serial]
async fn test_update_response_holds_the_whole_post_in_the_edit_context() {
    let ctx = create_test_context();
    let post_id = create_draft(&ctx.api, "Before").await;
    let fetched = fetch_raw(&ctx.api, post_id).await;
    let request_builder = WpApiRequestBuilder::new(
        test_site_api_url_resolver(),
        ctx.api.posts().get_delegate().auth_provider.clone(),
    );

    for (case, params) in [
        (
            "only the title",
            PostUpdateParams {
                title: Some("Only the title".to_string()),
                ..Default::default()
            },
        ),
        (
            "only the status",
            PostUpdateParams {
                status: Some(PostStatus::Pending),
                ..Default::default()
            },
        ),
        ("no fields", PostUpdateParams::default()),
    ] {
        let request = request_builder
            .posts()
            .update(&PostEndpointType::Posts, &post_id, &params);
        let response = ctx
            .api
            .posts()
            .perform(Arc::new(request), None)
            .await
            .expect("The update should get an answer");
        assert_eq!(response.status_code, 200, "{case}");
        let updated = json_body(&response.body);

        assert_eq!(
            keys(&updated),
            keys(&fetched),
            "{case}: the response's keys"
        );
        for field in ["title", "content", "excerpt", "guid"] {
            assert!(
                updated[field]["raw"].is_string(),
                "{case}: `{field}.raw` is only in the edit context"
            );
        }
    }

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn test_editor_save_pushes_the_working_copy_and_records_the_push() {
    let ctx = create_test_context();
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let local_content = open_local_content(&dir);
    let post_id = create_draft(&ctx.api, "Before").await;
    let service = Arc::new(PostEditorService::new(
        Arc::new(ctx.service),
        local_content.clone(),
        site(),
    ));

    let session = service
        .clone()
        .open_post(post_id)
        .await
        .expect("Opening the post should succeed");
    let response = session
        .perform_editor_request(EditorRequest {
            method: "PUT".to_string(),
            path: format!("/wp/v2/posts/{}?_locale=user", post_id.0),
            body: Some(
                serde_json::to_vec(&json!({ "id": post_id.0, "title": "Saved from the editor" }))
                    .expect("JSON serializes"),
            ),
        })
        .await
        .expect("The save should get an answer");

    assert_eq!(response.status_code, 200);
    let saved = json_body(&response.body);
    assert_eq!(saved["title"]["raw"], json!("Saved from the editor"));
    // Fields the editor didn't send are pushed as the working copy has them.
    assert_eq!(saved["excerpt"]["raw"], json!("An excerpt"));
    let on_server = ctx
        .api
        .posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Posts,
            &post_id,
            &PostRetrieveParams::default(),
        )
        .await
        .expect("Fetching the post should succeed")
        .data;
    assert_eq!(
        on_server.title.and_then(|title| title.raw),
        Some("Saved from the editor".to_string())
    );
    let working_copy = local_content
        .read_post_working_copy(session.working_copy_id())
        .expect("Read should succeed")
        .expect("The working copy exists");
    assert_eq!(
        working_copy
            .remote_target
            .remote_post
            .map(|remote| remote.base_modified_gmt),
        Some(on_server.modified_gmt)
    );
    session.end().expect("Ending the session should succeed");

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn test_new_post_is_created_on_its_first_push() {
    let ctx = create_test_context();
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let local_content = open_local_content(&dir);
    let service = Arc::new(PostEditorService::new(
        Arc::new(ctx.service),
        local_content,
        site(),
    ));

    let session = service
        .clone()
        .open_new_post()
        .expect("Opening a new post should succeed");
    session
        .record_change(
            serde_json::to_vec(&json!({
                "title": "Written on the device",
                "content": "<!-- wp:paragraph --><p>New</p><!-- /wp:paragraph -->",
                "status": "auto-draft",
                "date": null,
            }))
            .expect("JSON serializes"),
        )
        .expect("Recording should succeed");
    let response = session.push().await.expect("The push should get an answer");

    assert_eq!(response.status_code, 201);
    let post_id = session
        .remote_post_id()
        .expect("Read should succeed")
        .expect("The push created the post");
    let on_server = ctx
        .api
        .posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Posts,
            &post_id,
            &PostRetrieveParams::default(),
        )
        .await
        .expect("Fetching the post should succeed")
        .data;
    assert_eq!(on_server.status, PostStatus::Draft);
    assert_eq!(
        on_server.title.and_then(|title| title.raw),
        Some("Written on the device".to_string())
    );
    let editor_post = json_body(&session.editor_post().expect("Reading should succeed"));
    assert_eq!(editor_post["id"], json!(post_id.0));

    RestoreServer::db().await;
}
