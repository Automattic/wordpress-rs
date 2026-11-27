//! `PostEditorService` and `PostEditorSession` against a scripted server.

use super::*;
use crate::testing::EmptyAppNotifier;
use async_trait::async_trait;
use rusqlite::Connection;
use serde_json::json;
use std::collections::VecDeque;
use tempfile::TempDir;
use wp_api::date::WpGmtDateTime;
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
use wp_mobile_local_content::{
    WpLocalContentStore, db_types::editor_session::EditorRequestOutcome,
};
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

fn editor_request(
    request_id: &str,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> EditorRequest {
    EditorRequest {
        request_id: request_id.to_string(),
        method: method.to_string(),
        path: path.to_string(),
        headers: HashMap::from([("Authorization".to_string(), "Bearer secret".to_string())]),
        body: body.map(|body| serde_json::to_vec(&body).expect("JSON serializes")),
    }
}

fn editor_response(status_code: u32, body: &Value) -> EditorResponse {
    EditorResponse {
        status_code,
        headers: HashMap::new(),
        body: serde_json::to_vec(body).expect("JSON serializes"),
    }
}

fn date(value: &str) -> WpGmtDateTime {
    value.parse().expect("Date parses")
}

fn base_version(ctx: &Context, session: &PostEditorSession) -> Option<RemotePostReference> {
    ctx.local_content
        .read_post_working_copy(session.working_copy_id())
        .expect("Read should succeed")
        .expect("The working copy exists")
        .remote_target
        .remote_post
}

fn working_copy_title(ctx: &Context, session: &PostEditorSession) -> String {
    ctx.local_content
        .read_post_working_copy(session.working_copy_id())
        .expect("Read should succeed")
        .expect("The working copy exists")
        .fields
        .common
        .title
}

fn cached_modified_gmt(ctx: &Context, post_id: i64) -> Option<WpGmtDateTime> {
    ctx.posts
        .read_cached_post(PostId(post_id))
        .expect("Read should succeed")
        .map(|post| post.modified_gmt)
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
        .open_post(PostId(42), Some("identity-hash".to_string()))
        .await
        .expect("Opening the post should succeed")
}

/// Report that the editor sent `request` and got `response` to it.
async fn report(session: &PostEditorSession, request: EditorRequest, response: EditorResponse) {
    let request_id = request.request_id.clone();
    session
        .record_request_sent(request)
        .await
        .expect("Recording the request should succeed");
    session
        .record_request_response(request_id, response)
        .await
        .expect("Recording the response should succeed");
}

#[tokio::test]
async fn test_the_editor_opens_with_the_working_copy_as_the_latest_post() {
    let ctx = context();
    let session = open_post_42(&ctx).await;
    session
        .record_change(
            serde_json::to_vec(&json!({ "title": "The user's title" })).expect("JSON serializes"),
        )
        .await
        .expect("Recording should succeed");

    let editor_post = session.editor_post().expect("Reading should succeed");
    let post: Value = serde_json::from_slice(&editor_post.post).expect("The post is JSON");

    assert_eq!(editor_post.post_id, Some(PostId(42)));
    assert_eq!(post["title"], "The user's title");
    assert_eq!(post["content"], "<p>Content</p>");
    assert_eq!(
        ctx.local_content
            .read_editor_session(&session.id())
            .expect("Read should succeed")
            .and_then(|session| session.editor_identity),
        Some("identity-hash".to_string())
    );
}

#[tokio::test]
async fn test_a_new_post_opens_without_an_id() {
    let ctx = context();
    let session = ctx
        .service
        .clone()
        .open_new_post(None)
        .expect("Opening should succeed");

    let editor_post = session.editor_post().expect("Reading should succeed");

    assert_eq!(editor_post.post_id, None);
    assert!(ctx.server.requests().is_empty(), "Nothing was sent");
}

#[tokio::test]
async fn test_each_change_is_a_version_recorded_in_the_session() {
    let ctx = context();
    let session = open_post_42(&ctx).await;

    for title in ["One", "Two", "Two"] {
        session
            .record_change(serde_json::to_vec(&json!({ "title": title })).expect("JSON serializes"))
            .await
            .expect("Recording should succeed");
    }

    let sessions: Vec<_> = ctx
        .local_content
        .read_working_copy_versions(session.working_copy_id())
        .expect("Read should succeed")
        .into_iter()
        .map(|version| version.editor_session_id)
        .collect();
    // The version the working copy was created with, then one per change.
    assert_eq!(sessions, [None, Some(session.id()), Some(session.id())]);
}

#[tokio::test]
async fn test_an_editor_save_is_logged_recorded_and_taken_as_the_server_version() {
    let ctx = context();
    let session = open_post_42(&ctx).await;
    let saved = server_post(42, "Saved", "2024-02-01T00:00:00");

    let request = editor_request(
        "save",
        "PUT",
        "/wp/v2/posts/42",
        Some(json!({ "title": "Saved" })),
    );
    session
        .record_request_sent(request)
        .await
        .expect("Recording should succeed");
    // What the save sends is recorded before any response arrives.
    assert_eq!(working_copy_title(&ctx, &session), "Saved");

    session
        .record_request_response("save".to_string(), editor_response(200, &saved))
        .await
        .expect("Recording should succeed");

    assert_eq!(
        cached_modified_gmt(&ctx, 42),
        Some(date("2024-02-01T00:00:00"))
    );
    assert_eq!(
        base_version(&ctx, &session).map(|remote| remote.base_modified_gmt),
        Some(date("2024-02-01T00:00:00"))
    );
    assert_eq!(
        ctx.server.requests().len(),
        1,
        "wordpress-rs sent only the fetch when opening"
    );
    let requests = session.requests().expect("Reading should succeed");
    assert_eq!(requests.len(), 1);
    assert!(
        !requests[0].request.headers.contains_key("authorization"),
        "Credentials aren't stored"
    );
    assert!(matches!(
        &requests[0].outcome,
        Some(EditorRequestOutcome::Response { response, .. }) if response.status_code == 200
    ));
}

#[tokio::test]
async fn test_a_new_post_the_editor_creates_becomes_the_working_copy_remote_post() {
    let ctx = context();
    let session = ctx
        .service
        .clone()
        .open_new_post(None)
        .expect("Opening should succeed");

    report(
        &session,
        editor_request(
            "create",
            "POST",
            "/wp/v2/posts",
            Some(json!({ "title": "New" })),
        ),
        editor_response(201, &server_post(77, "New", "2024-02-01T00:00:00")),
    )
    .await;

    assert_eq!(
        session.remote_post_id().expect("Reading should succeed"),
        Some(PostId(77))
    );
    assert_eq!(working_copy_title(&ctx, &session), "New");
    assert_eq!(
        cached_modified_gmt(&ctx, 77),
        Some(date("2024-02-01T00:00:00"))
    );
}

#[tokio::test]
async fn test_a_read_updates_the_cache_but_not_the_base_version() {
    let ctx = context();
    let session = open_post_42(&ctx).await;
    let before = base_version(&ctx, &session);

    report(
        &session,
        editor_request("read", "GET", "/wp/v2/posts/42?context=edit", None),
        editor_response(
            200,
            &server_post(42, "Someone else's", "2024-03-01T00:00:00"),
        ),
    )
    .await;

    assert_eq!(
        cached_modified_gmt(&ctx, 42),
        Some(date("2024-03-01T00:00:00"))
    );
    assert_eq!(base_version(&ctx, &session), before);
}

#[tokio::test]
async fn test_responses_that_are_not_the_whole_current_post_are_only_logged() {
    let ctx = context();
    let session = open_post_42(&ctx).await;
    let before = base_version(&ctx, &session);

    // Older than what the cache has.
    report(
        &session,
        editor_request("older", "GET", "/wp/v2/posts/42?context=edit", None),
        editor_response(200, &server_post(42, "Old", "2024-01-10T00:00:00")),
    )
    .await;
    // Only some of the post's fields.
    report(
        &session,
        editor_request(
            "partial",
            "GET",
            "/wp/v2/posts/42?_fields=title,modified_gmt",
            None,
        ),
        editor_response(200, &server_post(42, "Partial", "2024-05-01T00:00:00")),
    )
    .await;
    // Refused by the server.
    report(
        &session,
        editor_request("refused", "PUT", "/wp/v2/posts/42", Some(json!({}))),
        editor_response(500, &json!({ "code": "internal_server_error" })),
    )
    .await;
    // Another post.
    report(
        &session,
        editor_request("other", "GET", "/wp/v2/posts/43?context=edit", None),
        editor_response(200, &server_post(43, "Other", "2024-05-01T00:00:00")),
    )
    .await;

    assert_eq!(
        cached_modified_gmt(&ctx, 42),
        Some(date("2024-01-16T00:00:00"))
    );
    assert_eq!(cached_modified_gmt(&ctx, 43), None);
    assert_eq!(base_version(&ctx, &session), before);
    assert_eq!(session.requests().expect("Reading should succeed").len(), 4);
}

#[tokio::test]
async fn test_a_request_that_got_no_answer_is_logged_as_such() {
    let ctx = context();
    let session = open_post_42(&ctx).await;

    session
        .record_request_sent(editor_request("offline", "PUT", "/wp/v2/posts/42", None))
        .await
        .expect("Recording should succeed");
    session
        .record_request_failure("offline".to_string(), "The device is offline".to_string())
        .await
        .expect("Recording should succeed");
    session
        .record_request_sent(editor_request("lost", "PUT", "/wp/v2/posts/42", None))
        .await
        .expect("Recording should succeed");

    let outcomes: Vec<_> = session
        .requests()
        .expect("Reading should succeed")
        .into_iter()
        .map(|record| record.outcome)
        .collect();
    assert!(matches!(
        outcomes.as_slice(),
        [Some(EditorRequestOutcome::Failure { .. }), None]
    ));
}

#[tokio::test]
async fn test_open_sessions_are_not_unfinished() {
    let ctx = context();
    let open = open_post_42(&ctx).await;
    let closed_id = {
        let crashed = ctx
            .service
            .clone()
            .open_new_post(None)
            .expect("Opening should succeed");
        crashed.id()
    };

    let unfinished: Vec<_> = ctx
        .service
        .read_unfinished_sessions()
        .expect("Reading should succeed")
        .into_iter()
        .map(|session| session.id)
        .collect();

    assert_eq!(unfinished, [closed_id]);
    open.end().expect("Ending should succeed");
}

#[tokio::test]
async fn test_a_push_after_the_editor_closes_sends_only_what_changed() {
    let ctx = context();
    let session = open_post_42(&ctx).await;
    session
        .record_change(
            serde_json::to_vec(&json!({ "title": "Offline edit" })).expect("JSON serializes"),
        )
        .await
        .expect("Recording should succeed");
    session.end().expect("Ending should succeed");
    serve_post_42_unchanged(&ctx);
    ctx.server.then(|sent| {
        response(
            sent,
            200,
            &server_post(42, "Offline edit", "2024-02-01T00:00:00"),
        )
    });

    ctx.service
        .push_working_copy(session.working_copy_id())
        .await
        .expect("The push should succeed");

    let update = ctx.server.requests().pop().expect("The update was sent");
    assert_eq!(update.body, Some(json!({ "title": "Offline edit" })));
    assert_eq!(
        base_version(&ctx, &session).map(|remote| remote.base_modified_gmt),
        Some(date("2024-02-01T00:00:00"))
    );
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

    let result = ctx
        .service
        .push_working_copy(session.working_copy_id())
        .await;

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
            serde_json::to_vec(&json!({ "title": "The user's title" })).expect("JSON serializes"),
        )
        .await
        .expect("Recording should succeed");
    let before = base_version(&ctx, &session);
    ctx.server.then(|sent| {
        response(
            sent,
            200,
            &server_post(42, "Someone else's title", "2024-06-01T00:00:00"),
        )
    });

    let result = ctx
        .service
        .push_working_copy(session.working_copy_id())
        .await;

    assert_eq!(
        result,
        Err(PostEditorError::Conflict {
            differing_fields: vec!["title".to_string()],
        })
    );
    assert_eq!(ctx.server.requests().len(), 2, "Nothing was pushed");
    assert_eq!(base_version(&ctx, &session), before);
}
