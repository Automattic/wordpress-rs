//! `PostEditorService` and `PostEditorSession` against a scripted server.

use super::*;
use crate::testing::EmptyAppNotifier;
use async_trait::async_trait;
use rusqlite::Connection;
use serde_json::json;
use std::collections::VecDeque;
use tempfile::TempDir;
use wp_api::{
    api_client::WpApiClientDelegate,
    auth::WpAuthenticationProvider,
    middleware::WpApiMiddlewarePipeline,
    parsed_url::ParsedUrl,
    request::{
        NetworkRequestAccessor, RequestContext, RequestExecutor, RequestMethod,
        WpMultipartFormRequest, WpNetworkHeaderMap,
        endpoint::{WpEndpointUrl, WpOrgSiteApiUrlResolver},
    },
};
use wp_mobile_cache::{
    MIGRATIONS, WpApiCache, db_types::self_hosted_site::SelfHostedSite,
    repository::sites::SiteRepository,
};
use wp_mobile_local_content::WpLocalContentStore;
use wp_mobile_sqlite::MigrationManager;

const SITE_URL: &str = "https://test.local";

/// A request the scripted server received.
#[derive(Debug, Clone)]
struct SentRequest {
    method: RequestMethod,
    url: String,
    body: Option<Value>,
}

type ScriptedResult = Result<WpNetworkResponse, RequestExecutionError>;
type ScriptedAnswer = Box<dyn FnOnce(&SentRequest) -> ScriptedResult + Send>;

/// Answers requests in order with the results it's given, and keeps the
/// requests.
#[derive(Default)]
struct ScriptedServer {
    results: Mutex<VecDeque<ScriptedAnswer>>,
    requests: Mutex<Vec<SentRequest>>,
}

impl ScriptedServer {
    fn then(&self, result: impl FnOnce(&SentRequest) -> ScriptedResult + Send + 'static) {
        self.results
            .lock()
            .expect("Lock isn't poisoned")
            .push_back(Box::new(result));
    }

    fn requests(&self) -> Vec<SentRequest> {
        self.requests.lock().expect("Lock isn't poisoned").clone()
    }
}

#[async_trait]
impl RequestExecutor for ScriptedServer {
    async fn execute(&self, request: Arc<WpNetworkRequest>) -> ScriptedResult {
        let sent = SentRequest {
            method: request.method(),
            url: request.url().0,
            body: request
                .body_as_string()
                .map(|body| serde_json::from_str(&body).expect("Request bodies are JSON")),
        };
        self.requests
            .lock()
            .expect("Lock isn't poisoned")
            .push(sent.clone());
        let result = self
            .results
            .lock()
            .expect("Lock isn't poisoned")
            .pop_front()
            .unwrap_or_else(|| panic!("No response scripted for {sent:?}"));
        result(&sent)
    }

    async fn upload(&self, _request: Arc<WpMultipartFormRequest>) -> ScriptedResult {
        unimplemented!("Uploads aren't scripted")
    }

    async fn sleep(&self, _: u64) {}

    fn cancel(&self, _: Arc<RequestContext>) {}
}

fn response(sent: &SentRequest, status_code: u32, body: &Value) -> ScriptedResult {
    Ok(WpNetworkResponse {
        body: serde_json::to_vec(body).expect("JSON serializes"),
        status_code,
        response_header_map: Arc::new(WpNetworkHeaderMap::default()),
        request_url: WpEndpointUrl(sent.url.clone()),
        request_method: sent.method.clone(),
        request_header_map: Arc::new(WpNetworkHeaderMap::default()),
    })
}

fn failure(sent: &SentRequest, reason: RequestExecutionErrorReason) -> ScriptedResult {
    Err(RequestExecutionError::RequestExecutionFailed {
        status_code: None,
        redirects: None,
        reason,
        request_url: sent.url.clone(),
        request_method: sent.method.clone(),
    })
}

/// A post as the server returns it with the edit context.
fn server_post(id: i64, title: &str, modified_gmt: &str) -> Value {
    json!({
        "id": id,
        "date": "2024-01-15T12:30:00",
        "date_gmt": "2024-01-15T10:30:00",
        "guid": { "raw": format!("{SITE_URL}/?p={id}"), "rendered": format!("{SITE_URL}/?p={id}") },
        "link": format!("{SITE_URL}/?p={id}"),
        "modified": modified_gmt,
        "modified_gmt": modified_gmt,
        "slug": "a-post",
        "status": "publish",
        "type": "post",
        "password": "",
        "permalink_template": format!("{SITE_URL}/%postname%/"),
        "generated_slug": "a-post",
        "title": { "raw": title, "rendered": title },
        "content": { "raw": "<p>Content</p>", "rendered": "<p>Content</p>", "protected": false, "block_version": 1 },
        "author": 1,
        "excerpt": { "raw": "", "rendered": "", "protected": false },
        "featured_media": 0,
        "comment_status": "open",
        "ping_status": "open",
        "sticky": false,
        "template": "",
        "format": "standard",
        "meta": { "footnotes": "" },
        "categories": [1],
        "tags": [],
        "plugin_field": "plugin value",
        "_links": { "self": [] },
    })
}

struct Context {
    server: Arc<ScriptedServer>,
    service: Arc<PostEditorService>,
    local_content: Arc<LocalContentService>,
    posts: Arc<PostService>,
    _dir: TempDir,
}

fn context() -> Context {
    let server = Arc::new(ScriptedServer::default());
    let api_url_resolver: Arc<dyn ApiUrlResolver> = Arc::new(WpOrgSiteApiUrlResolver::new(
        Arc::new(ParsedUrl::parse(&format!("{SITE_URL}/wp-json")).expect("URL parses")),
    ));
    let api_client = Arc::new(WpApiClient::new(
        api_url_resolver.clone(),
        WpApiClientDelegate {
            auth_provider: Arc::new(WpAuthenticationProvider::none()),
            request_executor: server.clone(),
            middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
            app_notifier: Arc::new(EmptyAppNotifier),
            language_provider: None,
        },
    ));

    let mut conn = Connection::open_in_memory().expect("Failed to open in-memory database");
    MigrationManager::new(&conn, &MIGRATIONS)
        .expect("Failed to create MigrationManager")
        .perform_migrations()
        .expect("Migrations should succeed");
    let db_site = SiteRepository
        .upsert_self_hosted_site(
            &mut conn,
            &SelfHostedSite {
                url: SITE_URL.to_string(),
                api_root: format!("{SITE_URL}/wp-json"),
            },
        )
        .expect("Site creation should succeed")
        .db_site;
    let cache = Arc::new(WpApiCache::try_from(conn).expect("Cache creation should succeed"));
    let posts = Arc::new(PostService::new(
        api_client.clone(),
        Arc::new(db_site),
        cache,
    ));

    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let store = WpLocalContentStore::new(
        dir.path()
            .join("local_content.db")
            .to_str()
            .expect("Temp path is UTF-8")
            .to_string(),
    )
    .expect("Opening the store should succeed");
    store
        .perform_migrations()
        .expect("Migrations should succeed");
    let local_content = Arc::new(LocalContentService::new(Arc::new(store)));

    let service = Arc::new(PostEditorService::from_parts(
        api_client,
        api_url_resolver,
        posts.clone(),
        local_content.clone(),
        SiteIdentity {
            url: Arc::new(ParsedUrl::parse(SITE_URL).expect("URL parses")),
            wordpress_com_site_id: None,
        },
    ));
    Context {
        server,
        service,
        local_content,
        posts,
        _dir: dir,
    }
}

fn editor_request(method: &str, path: &str, body: Option<Value>) -> EditorRequest {
    EditorRequest {
        method: method.to_string(),
        path: path.to_string(),
        body: body.map(|body| serde_json::to_vec(&body).expect("JSON serializes")),
    }
}

fn base_version(ctx: &Context, session: &PostEditorSession) -> Option<RemotePostReference> {
    ctx.local_content
        .read_post_working_copy(session.working_copy_id())
        .expect("Read should succeed")
        .expect("The working copy exists")
        .remote_target
        .remote_post
}

/// Answer the next request with post 42 as `open_post_42` fetched it, as the
/// fetch before a push finds it when nobody else changed it.
fn serve_post_42_unchanged(ctx: &Context) {
    ctx.server.then(|sent| {
        response(
            sent,
            200,
            &server_post(42, "On the server", "2024-01-16T00:00:00"),
        )
    });
}

async fn open_post_42(ctx: &Context) -> Arc<PostEditorSession> {
    serve_post_42_unchanged(ctx);
    ctx.service
        .clone()
        .open_post(PostId(42))
        .await
        .expect("Opening the post should succeed")
}

#[tokio::test]
async fn test_opening_a_post_fetches_it_and_shows_the_working_copy() {
    let ctx = context();
    let session = open_post_42(&ctx).await;
    session
        .record_change(
            serde_json::to_vec(
                &json!({ "title": { "raw": "Recorded" }, "plugin_field": "edited" }),
            )
            .expect("JSON serializes"),
        )
        .expect("Recording should succeed");

    let editor_post: Value =
        serde_json::from_slice(&session.editor_post().expect("Reading should succeed"))
            .expect("The editor's post is JSON");

    assert_eq!(
        ctx.server.requests().len(),
        1,
        "Only the first open fetches"
    );
    assert_eq!(editor_post["id"], json!(42));
    assert_eq!(
        editor_post["title"],
        json!({ "raw": "Recorded", "rendered": "On the server" })
    );
    assert_eq!(editor_post["plugin_field"], json!("edited"));
    assert_eq!(editor_post["meta"], json!({ "footnotes": "" }));
}

#[tokio::test]
async fn test_editor_save_records_the_edits_pushes_and_returns_the_response_unchanged() {
    let ctx = context();
    let session = open_post_42(&ctx).await;
    let saved = server_post(42, "Saved", "2024-02-01T00:00:00");
    let saved_for_server = saved.clone();
    serve_post_42_unchanged(&ctx);
    ctx.server
        .then(move |sent| response(sent, 200, &saved_for_server));

    let answer = session
        .perform_editor_request(editor_request(
            "PUT",
            "/wp/v2/posts/42?_locale=user",
            Some(json!({ "id": 42, "title": "Saved", "meta": { "plugin_meta": "1" } })),
        ))
        .await
        .expect("The save should succeed");

    assert_eq!(answer.status_code, 200);
    assert_eq!(
        answer.body,
        serde_json::to_vec(&saved).expect("JSON serializes")
    );
    let update = ctx.server.requests().pop().expect("The update was sent");
    assert_eq!(update.method, RequestMethod::POST);
    assert_eq!(update.url, format!("{SITE_URL}/wp-json/wp/v2/posts/42"));
    // The editor's edits are recorded first, and the update sends what
    // differs from the post as just fetched.
    assert_eq!(
        update.body,
        Some(json!({ "title": "Saved", "meta": { "plugin_meta": "1" } }))
    );
    assert_eq!(
        base_version(&ctx, &session).map(|remote| remote.base_modified_gmt),
        Some("2024-02-01T00:00:00".parse().expect("Date parses"))
    );
    assert_eq!(
        ctx.posts
            .read_cached_post(PostId(42))
            .expect("Read should succeed")
            .and_then(|post| post.title)
            .and_then(|title| title.raw),
        Some("Saved".to_string())
    );
    let session_record = ctx
        .local_content
        .read_editor_session(&session.id())
        .expect("Read should succeed")
        .expect("The session exists");
    assert!(session_record.last_recorded_at.is_some());
    assert_eq!(session_record.unconfirmed_push, None);
}

#[tokio::test]
async fn test_error_response_goes_back_as_it_is_and_records_no_push() {
    let ctx = context();
    let session = open_post_42(&ctx).await;
    let error =
        json!({ "code": "rest_cannot_edit", "message": "Sorry", "data": { "status": 403 } });
    let error_for_server = error.clone();
    serve_post_42_unchanged(&ctx);
    ctx.server
        .then(move |sent| response(sent, 403, &error_for_server));
    let before = base_version(&ctx, &session);

    let answer = session
        .perform_editor_request(editor_request(
            "PUT",
            "/wp/v2/posts/42",
            Some(json!({ "title": "Saved" })),
        ))
        .await
        .expect("An answer, even an error, isn't a failure");

    assert_eq!(answer.status_code, 403);
    assert_eq!(
        answer.body,
        serde_json::to_vec(&error).expect("JSON serializes")
    );
    assert_eq!(base_version(&ctx, &session), before);
}

#[tokio::test]
async fn test_new_post_is_created_on_its_first_push() {
    let ctx = context();
    let session = ctx
        .service
        .clone()
        .open_new_post()
        .expect("Opening a new post should succeed");
    session
        .record_change(
            serde_json::to_vec(&json!({ "title": "New", "status": "auto-draft" }))
                .expect("JSON serializes"),
        )
        .expect("Recording should succeed");
    assert!(
        ctx.server.requests().is_empty(),
        "Recording a change never pushes"
    );
    ctx.server
        .then(|sent| response(sent, 201, &server_post(77, "New", "2024-03-01T00:00:00")));

    session.push().await.expect("The push should succeed");

    let create = ctx.server.requests().pop().expect("The create was sent");
    assert_eq!(create.url, format!("{SITE_URL}/wp-json/wp/v2/posts"));
    let body = create.body.expect("The create has a body");
    assert_eq!(
        (&body["title"], &body["status"]),
        (&json!("New"), &json!("draft"))
    );
    assert_eq!(
        session.remote_post_id().expect("Read should succeed"),
        Some(PostId(77))
    );
}

#[tokio::test]
async fn test_push_that_was_never_sent_leaves_nothing_unconfirmed() {
    let ctx = context();
    let session = open_post_42(&ctx).await;
    serve_post_42_unchanged(&ctx);
    ctx.server.then(|sent| {
        failure(
            sent,
            RequestExecutionErrorReason::DeviceIsOfflineError {
                error_message: "offline".to_string(),
            },
        )
    });

    let result = session.push().await;

    assert!(
        matches!(result, Err(PostEditorError::NotSent { .. })),
        "{result:?}"
    );
    let record = ctx
        .local_content
        .read_editor_session(&session.id())
        .expect("Read should succeed")
        .expect("The session exists");
    assert_eq!(record.unconfirmed_push, None);
}

#[tokio::test]
async fn test_lost_push_is_settled_when_its_session_is_recovered() {
    let ctx = context();
    let session = open_post_42(&ctx).await;
    // `record_change` takes the whole post, as the editor has it.
    session
        .record_change(
            serde_json::to_vec(&server_post(42, "Lost", "2024-01-16T00:00:00"))
                .expect("JSON serializes"),
        )
        .expect("Recording should succeed");
    serve_post_42_unchanged(&ctx);
    ctx.server
        .then(|sent| failure(sent, RequestExecutionErrorReason::HttpTimeoutError));
    let result = session.push().await;
    assert!(
        matches!(result, Err(PostEditorError::MaybeSent { .. })),
        "{result:?}"
    );
    let session_id = session.id();
    // The app ends without ending the session.
    drop(session);

    // The push had landed: the server has what it sent.
    ctx.server
        .then(|sent| response(sent, 200, &server_post(42, "Lost", "2024-04-01T00:00:00")));
    let recovered = ctx
        .service
        .recover_unfinished_sessions()
        .await
        .expect("Recovery should succeed");

    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].id, session_id);
    assert_eq!(recovered[0].unconfirmed_push, None);
    let working_copy = ctx
        .local_content
        .read_post_working_copy(recovered[0].working_copy_id.clone())
        .expect("Read should succeed")
        .expect("The working copy exists");
    assert_eq!(
        working_copy
            .remote_target
            .remote_post
            .map(|remote| remote.base_modified_gmt),
        Some("2024-04-01T00:00:00".parse().expect("Date parses"))
    );
}

#[tokio::test]
async fn test_open_sessions_are_not_unfinished() {
    let ctx = context();
    let _open = open_post_42(&ctx).await;

    let recovered = ctx
        .service
        .recover_unfinished_sessions()
        .await
        .expect("Recovery should succeed");

    assert!(recovered.is_empty());
}

#[tokio::test]
async fn test_editor_requests_outside_the_designed_paths_are_refused() {
    let ctx = context();
    let session = open_post_42(&ctx).await;

    for (method, path, expected) in [
        ("POST", "/wp/v2/posts/42/autosaves", "NotDesigned"),
        ("GET", "/wp/v2/posts/42/revisions", "NotDesigned"),
        ("DELETE", "/wp/v2/posts/42", "NotDesigned"),
        ("PUT", "/wp/v2/posts/43", "AnotherPost"),
        ("GET", "/wp/v2/categories", "NotThePostEndpoint"),
    ] {
        let result = session
            .perform_editor_request(editor_request(method, path, None))
            .await;

        let kind = match result {
            Err(PostEditorError::NotDesigned { .. }) => "NotDesigned",
            Err(PostEditorError::AnotherPost { .. }) => "AnotherPost",
            Err(PostEditorError::NotThePostEndpoint { .. }) => "NotThePostEndpoint",
            other => panic!("{method} {path}: unexpected {other:?}"),
        };
        assert_eq!(kind, expected, "{method} {path}");
    }
    assert_eq!(
        ctx.server.requests().len(),
        1,
        "None of them reached the server"
    );
}

#[tokio::test]
async fn test_editor_read_fills_the_cache_and_returns_the_response_unchanged() {
    let ctx = context();
    let session = open_post_42(&ctx).await;
    session
        .record_change(
            serde_json::to_vec(&json!({ "title": "Not pushed" })).expect("JSON serializes"),
        )
        .expect("Recording should succeed");
    let on_server = server_post(42, "Changed on the server", "2024-05-01T00:00:00");
    let on_server_for_server = on_server.clone();
    ctx.server
        .then(move |sent| response(sent, 200, &on_server_for_server));

    let answer = session
        .perform_editor_request(editor_request("GET", "/wp/v2/posts/42?context=edit", None))
        .await
        .expect("The read should succeed");

    let read = ctx.server.requests().pop().expect("The read was sent");
    assert_eq!(
        read.url,
        format!("{SITE_URL}/wp-json/wp/v2/posts/42?context=edit")
    );
    // The editor is the source of truth while it's open, so the working copy
    // isn't laid over the server's answer.
    assert_eq!(
        answer.body,
        serde_json::to_vec(&on_server).expect("JSON serializes")
    );
    assert_eq!(
        ctx.posts
            .read_cached_post(PostId(42))
            .expect("Read should succeed")
            .and_then(|post| post.title)
            .and_then(|title| title.raw),
        Some("Changed on the server".to_string())
    );
}

#[tokio::test]
async fn test_lost_push_that_did_not_land_is_not_recorded() {
    let ctx = context();
    let session = open_post_42(&ctx).await;
    session
        .record_change(
            serde_json::to_vec(&server_post(42, "Lost", "2024-01-16T00:00:00"))
                .expect("JSON serializes"),
        )
        .expect("Recording should succeed");
    serve_post_42_unchanged(&ctx);
    ctx.server
        .then(|sent| failure(sent, RequestExecutionErrorReason::HttpTimeoutError));
    let _ = session.push().await;
    let before = base_version(&ctx, &session);
    drop(session);

    // Someone else changed the post, so the server doesn't have what was sent.
    ctx.server.then(|sent| {
        response(
            sent,
            200,
            &server_post(42, "Someone else's title", "2024-04-01T00:00:00"),
        )
    });
    let recovered = ctx
        .service
        .recover_unfinished_sessions()
        .await
        .expect("Recovery should succeed");

    assert_eq!(recovered[0].unconfirmed_push, None);
    let working_copy = ctx
        .local_content
        .read_post_working_copy(recovered[0].working_copy_id.clone())
        .expect("Read should succeed")
        .expect("The working copy exists");
    assert_eq!(working_copy.remote_target.remote_post, before);
}

#[tokio::test]
async fn test_push_after_a_failed_fetch_is_not_sent() {
    let ctx = context();
    let session = open_post_42(&ctx).await;
    ctx.server.then(|sent| {
        failure(
            sent,
            RequestExecutionErrorReason::ConnectionError {
                reason: "refused".to_string(),
            },
        )
    });

    let result = session.push().await;

    assert!(
        matches!(result, Err(PostEditorError::NotSent { .. })),
        "{result:?}"
    );
    assert_eq!(ctx.server.requests().len(), 2, "Only the fetches were sent");
}

#[tokio::test]
async fn test_push_is_refused_when_the_server_and_the_working_copy_both_changed() {
    let ctx = context();
    let session = open_post_42(&ctx).await;
    session
        .record_change(
            serde_json::to_vec(&server_post(42, "The user's title", "2024-01-16T00:00:00"))
                .expect("JSON serializes"),
        )
        .expect("Recording should succeed");
    let before = base_version(&ctx, &session);
    ctx.server.then(|sent| {
        response(
            sent,
            200,
            &server_post(42, "Someone else's title", "2024-06-01T00:00:00"),
        )
    });

    let result = session.push().await;

    assert_eq!(
        result,
        Err(PostEditorError::Conflict {
            differing_fields: vec!["title".to_string()],
        })
    );
    assert_eq!(ctx.server.requests().len(), 2, "Nothing was pushed");
    assert_eq!(base_version(&ctx, &session), before);
}

#[tokio::test]
async fn test_push_settles_a_lost_push_that_landed_instead_of_reporting_a_conflict() {
    let ctx = context();
    let session = open_post_42(&ctx).await;
    session
        .record_change(
            serde_json::to_vec(&server_post(42, "Lost", "2024-01-16T00:00:00"))
                .expect("JSON serializes"),
        )
        .expect("Recording should succeed");
    serve_post_42_unchanged(&ctx);
    ctx.server
        .then(|sent| failure(sent, RequestExecutionErrorReason::HttpTimeoutError));
    let _ = session.push().await;

    // The lost push had landed, so the server changed, but only by it.
    ctx.server
        .then(|sent| response(sent, 200, &server_post(42, "Lost", "2024-07-01T00:00:00")));
    ctx.server
        .then(|sent| response(sent, 200, &server_post(42, "Lost", "2024-07-01T00:00:01")));
    session.push().await.expect("The push should succeed");

    let update = ctx.server.requests().pop().expect("The update was sent");
    assert_eq!(
        update.body,
        Some(json!({})),
        "Nothing differs from the server"
    );
    assert_eq!(
        base_version(&ctx, &session).map(|remote| remote.base_modified_gmt),
        Some("2024-07-01T00:00:01".parse().expect("Date parses"))
    );
}
