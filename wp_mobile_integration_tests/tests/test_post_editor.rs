//! Recording what an editor does through `PostEditorService`, and pushing
//! after it closes, against the test server.

use serde_json::{Value, json};
use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};
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
    post_editor::{PostEditorService, PostEditorSession},
};
use wp_mobile_integration_tests::*;
use wp_mobile_local_content::{
    WpLocalContentStore,
    db_types::{
        editing_session::{SessionRequest, SessionResponse},
        working_copy::{PostWorkingCopyFields, PublishDate},
        working_copy_remote_target::SiteIdentity,
    },
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
    Arc::new(LocalContentService::new(
        Arc::new(store),
        "test-app".to_string(),
    ))
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

/// Send a request the way the editor does, with the test site's
/// credentials, and report it to `session` as the editor would: before it's
/// sent, and with its response. Returns the response's JSON body.
async fn editor_sends(
    api: &WpApiClient,
    session: &PostEditorSession,
    request_id: &str,
    post_id: Option<PostId>,
    body: Value,
) -> (u32, Value) {
    let segments = std::iter::once("posts".to_string())
        .chain(post_id.map(|post_id| post_id.0.to_string()))
        .collect::<Vec<_>>();
    let path = format!("/wp/v2/{}", segments.join("/"));
    let url = test_site_api_url_resolver()
        .resolve("wp/v2".to_string(), segments)
        .inner
        .clone();
    let auth_provider = api.posts().get_delegate().auth_provider.clone();
    let body = body
        .as_object()
        .cloned()
        .expect("The body is a JSON object");
    let request = InnerRequestBuilder::new(auth_provider).post(url.into(), Some(&body));

    session
        .record_request_sent(SessionRequest {
            request_id: request_id.to_string(),
            method: "POST".to_string(),
            path,
            headers: HashMap::new(),
            body: Some(serde_json::to_vec(&body).expect("JSON serializes")),
        })
        .await
        .expect("Recording the request should succeed");
    let response = api
        .posts()
        .perform(Arc::new(request), None)
        .await
        .expect("The request should get an answer");
    let status_code = response.status_code;
    let json = json_body(&response.body);
    session
        .record_request_response(
            request_id.to_string(),
            SessionResponse {
                status_code,
                headers: HashMap::new(),
                body: response.body,
            },
        )
        .await
        .expect("Recording the response should succeed");
    (status_code, json)
}

async fn fetch_post(api: &WpApiClient, post_id: PostId) -> wp_api::posts::AnyPostWithEditContext {
    api.posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Posts,
            &post_id,
            &PostRetrieveParams::default(),
        )
        .await
        .expect("Fetching the post should succeed")
        .data
}

#[tokio::test]
#[serial]
async fn test_editor_save_is_recorded_and_its_response_becomes_the_base_version() {
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
        .open_post(post_id, None, None)
        .await
        .expect("Opening the post should succeed");

    let (status_code, saved) = editor_sends(
        &ctx.api,
        &session,
        "save",
        Some(post_id),
        json!({ "title": "Saved from the editor" }),
    )
    .await;

    assert_eq!(status_code, 200);
    assert_eq!(saved["title"]["raw"], json!("Saved from the editor"));
    let on_server = fetch_post(&ctx.api, post_id).await;
    let working_copy = local_content
        .read_post_working_copy(session.working_copy_id())
        .expect("Read should succeed")
        .expect("The working copy exists");
    assert_eq!(working_copy.fields.common.title, "Saved from the editor");
    assert_eq!(
        working_copy
            .remote_target
            .remote_post
            .map(|remote| remote.base_modified_gmt),
        Some(on_server.modified_gmt)
    );
    let requests = session.requests().expect("Reading should succeed");
    assert_eq!(requests.len(), 1);
    assert!(requests[0].outcome.is_some());
    session.end().expect("Ending the session should succeed");

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn test_new_post_the_editor_creates_becomes_the_working_copy_remote_post() {
    let ctx = create_test_context();
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let service = Arc::new(PostEditorService::new(
        Arc::new(ctx.service),
        open_local_content(&dir),
        site(),
    ));
    let session = service
        .clone()
        .open_new_post(None, None)
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
        .await
        .expect("Recording should succeed");

    let (status_code, created) = editor_sends(
        &ctx.api,
        &session,
        "create",
        None,
        json!({
            "title": "Written on the device",
            "content": "<!-- wp:paragraph --><p>New</p><!-- /wp:paragraph -->",
            "status": "draft",
        }),
    )
    .await;

    assert_eq!(status_code, 201);
    let post_id = PostId(created["id"].as_i64().expect("The response has an ID"));
    assert_eq!(
        session.remote_post_id().expect("Read should succeed"),
        Some(post_id)
    );
    assert_eq!(
        session
            .editor_post()
            .expect("Reading should succeed")
            .post_id,
        Some(post_id)
    );

    RestoreServer::db().await;
}

/// The working copy keeps the editor's `date` in the site's timezone, and a
/// push after the editor closes sends it as it is. `null` makes the date float
/// again.
#[tokio::test]
#[serial]
async fn test_editor_date_is_pushed_in_the_site_timezone_and_null_floats_it() {
    let ctx = create_test_context();
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let post_id = create_draft(&ctx.api, "Dated").await;
    let service = Arc::new(PostEditorService::new(
        Arc::new(ctx.service),
        open_local_content(&dir),
        site(),
    ));
    let session = service
        .clone()
        .open_post(post_id, None, None)
        .await
        .expect("Opening the post should succeed");
    let change_date_and_push = |date: Value| {
        let service = service.clone();
        let session = session.clone();
        async move {
            session
                .record_change(
                    serde_json::to_vec(&json!({ "date": date })).expect("JSON serializes"),
                )
                .await
                .expect("Recording should succeed");
            json_body(
                &service
                    .push_working_copy(session.working_copy_id())
                    .await
                    .expect("The push should get an answer")
                    .body,
            )
        }
    };

    let dated = change_date_and_push(json!("2030-01-15T10:30:00")).await;
    assert_eq!(dated["date"], json!("2030-01-15T10:30:00"));
    assert_ne!(
        dated["date_gmt"], dated["date"],
        "The test site's timezone isn't UTC, so the server converts the date"
    );

    let floating: wp_api::posts::AnyPostWithEditContext =
        serde_json::from_value(change_date_and_push(Value::Null).await)
            .expect("The response is a post");
    assert_eq!(
        PostWorkingCopyFields::from_remote(&floating)
            .expect("Conversion should succeed")
            .common
            .publish_date,
        PublishDate::Immediately
    );

    RestoreServer::db().await;
}

/// A post someone else changed isn't a conflict when the working copy already
/// has what the server has: every other field converted from the fetched post
/// equals the working copy's, so nothing differs.
#[tokio::test]
#[serial]
async fn test_push_matching_the_server_is_not_a_conflict() {
    let ctx = create_test_context();
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let local_content = open_local_content(&dir);
    let post_id = create_draft(&ctx.api, "Before").await;
    let other_client = WpApiClient::new(test_site_api_url_resolver(), api_client_delegate());
    let service = Arc::new(PostEditorService::new(
        Arc::new(ctx.service),
        local_content.clone(),
        site(),
    ));
    let session = service
        .clone()
        .open_post(post_id, None, None)
        .await
        .expect("Opening the post should succeed");

    // The modified date has a resolution of a second.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let changed = other_client
        .posts()
        .update(
            &PostEndpointType::Posts,
            &post_id,
            &PostUpdateParams {
                excerpt: Some("Changed elsewhere".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("The other update should succeed")
        .data;
    let working_copy = local_content
        .read_post_working_copy(session.working_copy_id())
        .expect("Read should succeed")
        .expect("The working copy exists");
    let mut fields = working_copy.fields.clone();
    fields.common.excerpt = "Changed elsewhere".to_string();
    local_content
        .save_post_working_copy(working_copy.id, fields)
        .expect("Saving should succeed");

    let response = service
        .push_working_copy(session.working_copy_id())
        .await
        .expect("The push should succeed");

    assert_eq!(response.status_code, 200);
    let base = local_content
        .read_post_working_copy(session.working_copy_id())
        .expect("Read should succeed")
        .expect("The working copy exists")
        .remote_target
        .remote_post
        .map(|remote| remote.base_modified_gmt)
        .expect("The post is on the server");
    assert!(
        base.0 >= changed.modified_gmt.0,
        "The base version {base:?} is at least the other client's {:?}",
        changed.modified_gmt
    );

    RestoreServer::db().await;
}
