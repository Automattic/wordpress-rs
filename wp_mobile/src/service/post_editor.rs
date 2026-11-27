//! Editing posts in a block editor that makes its own requests for the post,
//! with wordpress-rs recording what the editor does.
//!
//! While the editor is open, it fetches and saves the post itself, and reports
//! to the app what it did. The app passes each report to a
//! `PostEditorSession`, which records it: a change to the post becomes a
//! version of the post's working copy, and a request becomes an entry in the
//! session's request log, completed when its response or failure is reported.
//! Information flows one way, from the editor to wordpress-rs, apart from the
//! post the editor opens with.
//!
//! The choices still open in the design are marked `[decision]`, each in one
//! place.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use futures::lock::Mutex as AsyncMutex;
use serde_json::{Map, Value};
use wp_api::{
    WpAdditionalFields,
    api_client::{IsWpApiClientDelegate, WpApiClient},
    api_error::{RequestExecutionError, RequestExecutionErrorReason},
    middleware::PerformsRequests,
    posts::{AnyPostWithEditContext, PostId, PostMeta},
    request::{
        InnerRequestBuilder, NetworkRequestAccessor, WpNetworkHeaderMap, WpNetworkRequest,
        WpNetworkResponse,
        endpoint::{ApiEndpointUrl, ApiUrlResolver},
    },
};
use wp_mobile_local_content::{
    LocalContentError,
    db_types::{
        editing_session::{
            ActivitySource, EditingSessionId, SessionRequest, SessionRequestRecord, SessionResponse,
        },
        working_copy::{
            FeaturedImage, PostWorkingCopy, PostWorkingCopyFields, PublishDate, StatusSetting,
            WorkingCopyCommonFields, WorkingCopyId,
        },
        working_copy_remote_target::{RemotePostReference, SiteIdentity, WorkingCopyRemoteTarget},
    },
    editor::EditorPostConversionError,
    remote::RemotePostConversionError,
};

use crate::service::{WpService, local_content::LocalContentService, posts::PostService};

/// What the editor opens with: the working copy, as the latest version of the
/// post. The editor fetches the server's version itself.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EditorPost {
    /// The post's ID on the server, or `None` for a post that isn't on the
    /// server yet.
    pub post_id: Option<PostId>,
    /// Every field of the working copy, as a JSON object in the shape the REST
    /// API takes it in a write.
    pub post: Vec<u8>,
}

/// The server's response, as it arrived.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ServerResponse {
    pub status_code: u32,
    /// Header names in lowercase. A header with several values has them
    /// joined with `, `.
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum PostEditorError {
    #[error("Local content error: {reason}")]
    LocalContent { reason: String },
    #[error("Cache error: {reason}")]
    Cache { reason: String },
    #[error("No post working copy with id {}", id.0)]
    WorkingCopyNotFound { id: WorkingCopyId },
    #[error("The editor's post can't be recorded: {reason}")]
    InvalidEditorPost { reason: String },
    #[error("The post from the server can't be edited: {reason}")]
    UneditableRemotePost { reason: String },
    #[error("Fetching the post failed: {reason}")]
    FetchFailed { reason: String },
    /// The request never reached the server.
    #[error("The request wasn't sent: {reason}")]
    NotSent { reason: String },
    /// The request got no answer, and may have reached the server.
    #[error("The request may have been sent: {reason}")]
    MaybeSent { reason: String },
    /// The server reported success, but its response isn't a post.
    #[error("The server's response to a push isn't a post: {reason}")]
    UnreadablePushResponse { status_code: u32, reason: String },
    /// The post changed on the server since the working copy's base version,
    /// and the working copy differs from it in `differing_fields`. Which side
    /// changed each field isn't known, since the cache holds only the server's
    /// latest version. Nothing was sent.
    #[error("The post changed on the server, and differs in {differing_fields:?}")]
    Conflict { differing_fields: Vec<String> },
}

impl From<LocalContentError> for PostEditorError {
    fn from(err: LocalContentError) -> Self {
        match err {
            LocalContentError::WorkingCopyNotFound { id } => Self::WorkingCopyNotFound { id },
            err => Self::LocalContent {
                reason: err.to_string(),
            },
        }
    }
}

impl From<wp_mobile_cache::SqliteDbError> for PostEditorError {
    fn from(err: wp_mobile_cache::SqliteDbError) -> Self {
        Self::Cache {
            reason: err.to_string(),
        }
    }
}

impl From<EditorPostConversionError> for PostEditorError {
    fn from(err: EditorPostConversionError) -> Self {
        Self::InvalidEditorPost {
            reason: err.to_string(),
        }
    }
}

impl From<RemotePostConversionError> for PostEditorError {
    fn from(err: RemotePostConversionError) -> Self {
        Self::UneditableRemotePost {
            reason: err.to_string(),
        }
    }
}

/// Editing a site's posts: what the editor does is recorded through here.
///
/// Open a post with `open_post` or `open_new_post`, which start an editing
/// session, and keep the returned `PostEditorSession` while the editor is
/// open. After the editor closes, push a working copy with
/// `push_working_copy`.
#[derive(uniffi::Object)]
pub struct PostEditorService {
    api_client: Arc<WpApiClient>,
    api_url_resolver: Arc<dyn ApiUrlResolver>,
    posts: Arc<PostService>,
    local_content: Arc<LocalContentService>,
    site: SiteIdentity,
    push_locks: Mutex<HashMap<WorkingCopyId, Arc<AsyncMutex<()>>>>,
}

#[uniffi::export]
impl PostEditorService {
    /// Create the service for the site of `wp_service`. `site` identifies the
    /// same site for its working copies.
    #[uniffi::constructor]
    pub fn new(
        wp_service: Arc<WpService>,
        local_content: Arc<LocalContentService>,
        site: SiteIdentity,
    ) -> Self {
        Self::from_parts(
            wp_service.api_client.clone(),
            wp_service.api_url_resolver.clone(),
            wp_service.posts.clone(),
            local_content,
            site,
        )
    }

    /// Start an editing session for post `post_id` from the server, in the
    /// editor named `editor_name`, such as GutenbergKit.
    ///
    /// Uses the post's working copy if it has one. Otherwise creates one from
    /// the post in the cache, fetching the post if the cache doesn't have it.
    ///
    /// `identity` is who the editor makes its requests as, in a form that
    /// isn't a credential, such as a hash of its token. It's recorded on the
    /// session as given, and nothing else uses it yet.
    pub async fn open_post(
        self: Arc<Self>,
        post_id: PostId,
        editor_name: Option<String>,
        identity: Option<String>,
    ) -> Result<Arc<PostEditorSession>, PostEditorError> {
        let working_copy = match self
            .local_content
            .read_post_working_copy_for_remote_post(self.site.clone(), post_id)?
        {
            Some(working_copy) => working_copy,
            None => {
                // [decision] The fetch that creates a working copy isn't
                // logged: a session can't exist before its working copy.
                let server_post = match self.posts.read_cached_post(post_id)? {
                    Some(post) => post,
                    None => self.fetch_post(post_id, None).await?,
                };
                self.local_content.create_post_working_copy(
                    WorkingCopyRemoteTarget {
                        site: self.site.clone(),
                        remote_post: Some(RemotePostReference::from_remote(&server_post)),
                    },
                    PostWorkingCopyFields::from_remote(&server_post)?,
                )?
            }
        };
        self.start_session(
            working_copy.id,
            ActivitySource::Editor { name: editor_name },
            identity,
        )
    }

    /// Start an editing session for a new post, in the editor named
    /// `editor_name`: a working copy with no remote post. The editor creates
    /// the post on the server when it first saves it, and reporting that save
    /// links the working copy to the new post.
    ///
    /// `identity` is recorded as `open_post` records it.
    pub fn open_new_post(
        self: Arc<Self>,
        editor_name: Option<String>,
        identity: Option<String>,
    ) -> Result<Arc<PostEditorSession>, PostEditorError> {
        let working_copy = self.local_content.create_post_working_copy(
            WorkingCopyRemoteTarget {
                site: self.site.clone(),
                remote_post: None,
            },
            empty_post_fields(),
        )?;
        self.start_session(
            working_copy.id,
            ActivitySource::Editor { name: editor_name },
            identity,
        )
    }

    /// Send a working copy to the server, creating the post there if it
    /// doesn't exist yet, and return the server's response.
    ///
    /// Use it after the editor has closed. While it's open, the editor saves
    /// the post itself.
    ///
    /// The push is a one-off editing session of its own, named `push`, so its
    /// requests are in that session's request log. Updating a post fetches it
    /// first, then sends only the fields that differ from the server's. Fails
    /// with `Conflict` if the post changed on the server since the base
    /// version and the working copy differs from it, and with `NotSent` or
    /// `MaybeSent` if no answer came. Any answer, error statuses included, is
    /// returned. On a successful one, the post goes into the cache and becomes
    /// the working copy's base version.
    pub async fn push_working_copy(
        &self,
        working_copy_id: WorkingCopyId,
    ) -> Result<ServerResponse, PostEditorError> {
        let session = self.local_content.create_editing_session(
            working_copy_id.clone(),
            ActivitySource::App {
                name: Some("push".to_string()),
            },
            None,
        )?;
        let result = self.push(&working_copy_id, &session.id).await;
        self.local_content.end_editing_session(session.id)?;
        result
    }
}

impl PostEditorService {
    fn from_parts(
        api_client: Arc<WpApiClient>,
        api_url_resolver: Arc<dyn ApiUrlResolver>,
        posts: Arc<PostService>,
        local_content: Arc<LocalContentService>,
        site: SiteIdentity,
    ) -> Self {
        Self {
            api_client,
            api_url_resolver,
            posts,
            local_content,
            site,
            push_locks: Mutex::new(HashMap::new()),
        }
    }

    fn start_session(
        self: Arc<Self>,
        working_copy_id: WorkingCopyId,
        source: ActivitySource,
        identity: Option<String>,
    ) -> Result<Arc<PostEditorSession>, PostEditorError> {
        let session = self.local_content.create_editing_session(
            working_copy_id.clone(),
            source.clone(),
            identity,
        )?;
        Ok(Arc::new(PostEditorSession {
            service: self,
            id: session.id,
            working_copy_id,
            source,
            reports: AsyncMutex::new(()),
        }))
    }

    fn read_working_copy(&self, id: &WorkingCopyId) -> Result<PostWorkingCopy, PostEditorError> {
        self.local_content
            .read_post_working_copy(id.clone())?
            .ok_or_else(|| PostEditorError::WorkingCopyNotFound { id: id.clone() })
    }

    /// Send one of wordpress-rs's own requests. In a session, the request and
    /// its response or failure go into the session's request log.
    ///
    /// The request is sent even if logging it fails, and the failure is
    /// logged as a warning.
    async fn send(
        &self,
        request: WpNetworkRequest,
        session: Option<&EditingSessionId>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        let source = ActivitySource::App { name: None };
        let request_id = request.request_id();
        if let Some(session) = session {
            let url = url::Url::parse(&request.url().0).ok();
            let logged = SessionRequest {
                request_id: request_id.clone(),
                method: format!("{:?}", request.method()),
                path: url
                    .map(|url| match url.query() {
                        Some(query) => format!("{}?{query}", url.path()),
                        None => url.path().to_string(),
                    })
                    .unwrap_or_else(|| request.url().0),
                headers: string_headers(&request.header_map()),
                body: request.body_as_string().map(String::into_bytes),
            };
            if let Err(err) = self
                .local_content
                .record_session_request_sent(session, &source, &logged)
            {
                log::warn!("Logging a request in session {} failed: {err}", session.0);
            }
        }
        let result = self
            .api_client
            .posts()
            .perform(Arc::new(request), None)
            .await;
        if let Some(session) = session {
            let logged = match &result {
                Ok(response) => self
                    .local_content
                    .record_session_request_response(
                        session,
                        &request_id,
                        &SessionResponse {
                            status_code: response.status_code,
                            headers: string_headers(&response.response_header_map),
                            body: response.body.clone(),
                        },
                    )
                    .map(|_| ()),
                Err(err) => self.local_content.record_session_request_failure(
                    session,
                    &request_id,
                    &err.to_string(),
                ),
            };
            if let Err(err) = logged {
                log::warn!("Logging a response in session {} failed: {err}", session.0);
            }
        }
        result
    }

    /// Fetch a post with the edit context and put it in the cache. In a
    /// session, the fetch is logged in it.
    async fn fetch_post(
        &self,
        post_id: PostId,
        session: Option<&EditingSessionId>,
    ) -> Result<AnyPostWithEditContext, PostEditorError> {
        let mut url = self
            .api_url_resolver
            .resolve(
                "wp/v2".to_string(),
                vec!["posts".to_string(), post_id.0.to_string()],
            )
            .inner
            .clone();
        url.query_pairs_mut().append_pair("context", "edit");
        let request =
            InnerRequestBuilder::new(self.api_client.posts().get_delegate().auth_provider.clone())
                .get(ApiEndpointUrl::new(url));
        let fetch_failed = |reason: String| PostEditorError::FetchFailed { reason };
        let response = self
            .send(request, session)
            .await
            .map_err(|err| fetch_failed(err.to_string()))?;
        if !is_success(response.status_code) {
            return Err(fetch_failed(format!(
                "The server answered with status {}",
                response.status_code
            )));
        }
        let post: AnyPostWithEditContext =
            serde_json::from_slice(&response.body).map_err(|err| fetch_failed(err.to_string()))?;
        self.posts.cache_post_from_server(&post)?;
        Ok(post)
    }

    /// A request that creates a post with `body`, or updates `post_id` with it.
    fn write_request(
        &self,
        post_id: Option<PostId>,
        body: &Map<String, Value>,
    ) -> WpNetworkRequest {
        let segments = std::iter::once("posts".to_string())
            .chain(post_id.map(|post_id| post_id.0.to_string()))
            .collect();
        let url = self
            .api_url_resolver
            .resolve("wp/v2".to_string(), segments)
            .inner
            .clone();
        InnerRequestBuilder::new(self.api_client.posts().get_delegate().auth_provider.clone())
            .post(ApiEndpointUrl::new(url), Some(body))
    }

    /// Push a working copy in `session`, and return the server's response.
    async fn push(
        &self,
        working_copy_id: &WorkingCopyId,
        session: &EditingSessionId,
    ) -> Result<ServerResponse, PostEditorError> {
        // [decision] One push at a time per working copy: a push waits for one
        // already in flight.
        let lock = self
            .push_locks
            .lock()
            .expect("The push locks aren't poisoned")
            .entry(working_copy_id.clone())
            .or_default()
            .clone();
        let _in_flight = lock.lock().await;
        let working_copy = self.read_working_copy(working_copy_id)?;
        let (remote_post, body) = match working_copy.remote_target.remote_post {
            // A new post: every field goes into the create.
            None => (None, working_copy.fields.to_rest_fields()),
            Some(remote_post) => {
                let changes = self
                    .changes_to_push(&working_copy, remote_post, session)
                    .await?;
                (Some(remote_post), changes)
            }
        };
        let request = self.write_request(remote_post.map(|remote_post| remote_post.post_id), &body);

        let response = self
            .send(request, Some(session))
            .await
            .map_err(push_failure)?;
        if !is_success(response.status_code) {
            return Ok(ServerResponse::from(response));
        }

        let post: AnyPostWithEditContext =
            serde_json::from_slice(&response.body).map_err(|err| {
                PostEditorError::UnreadablePushResponse {
                    status_code: response.status_code,
                    reason: err.to_string(),
                }
            })?;
        self.posts.cache_post_from_server(&post)?;
        // The base version comes from this response, not a later fetch, which
        // could already include someone else's change.
        self.local_content.record_post_working_copy_push(
            working_copy_id.clone(),
            RemotePostReference::from_remote(&post),
        )?;
        Ok(ServerResponse::from(response))
    }

    /// Fetch the post a working copy updates, in `session`, and return the
    /// fields that differ from the server's.
    ///
    /// Fails with `Conflict` if the post changed on the server since the base
    /// version and the working copy differs from it, and with `NotSent` if
    /// the fetch failed.
    async fn changes_to_push(
        &self,
        working_copy: &PostWorkingCopy,
        remote_post: RemotePostReference,
        session: &EditingSessionId,
    ) -> Result<Map<String, Value>, PostEditorError> {
        // [decision] Fetch the post first, to check for a conflict and to send
        // only what changed.
        let server_post = self
            .fetch_post(remote_post.post_id, Some(session))
            .await
            .map_err(|err| PostEditorError::NotSent {
                reason: format!("Fetching the post before pushing failed: {err}"),
            })?;
        let server_fields = PostWorkingCopyFields::from_remote(&server_post)?;
        // [decision] Send only the fields that differ from the server's.
        let changes = working_copy.fields.rest_fields_changed_from(&server_fields);

        if server_post.modified_gmt != remote_post.base_modified_gmt {
            if changes.is_empty() {
                // The server already has what the user has: nothing conflicts.
                self.local_content.record_post_working_copy_push(
                    working_copy.id.clone(),
                    RemotePostReference::from_remote(&server_post),
                )?;
                return Ok(changes);
            }
            // [not designed] What happens to a conflict. The working copy keeps
            // everything, and nothing is sent.
            return Err(PostEditorError::Conflict {
                differing_fields: changes.into_iter().map(|(key, _)| key).collect(),
            });
        }
        Ok(changes)
    }
}

/// One editing session in an editor: a post open in the editor, from opening
/// it to closing it.
///
/// While the editor is open, it's the source of truth, and it makes its own
/// requests for the post. Pass each of its reports here: `record_change` for a
/// change to the post, `record_request_sent` when it sends a request, and
/// `record_request_response` or `record_request_failure` when the request
/// completes. Call `end` when the editor closes.
///
/// Reports are recorded one at a time, in the order they arrive.
#[derive(uniffi::Object)]
pub struct PostEditorSession {
    service: Arc<PostEditorService>,
    id: EditingSessionId,
    working_copy_id: WorkingCopyId,
    /// The editor, which sends the requests reported here.
    source: ActivitySource,
    /// Held while a report is recorded, so the working copy's versions and
    /// the request log follow the order the reports arrived in.
    reports: AsyncMutex<()>,
}

#[uniffi::export]
impl PostEditorSession {
    pub fn id(&self) -> EditingSessionId {
        self.id.clone()
    }

    pub fn working_copy_id(&self) -> WorkingCopyId {
        self.working_copy_id.clone()
    }

    /// The post's ID on the server, or `None` if it isn't on the server yet.
    pub fn remote_post_id(&self) -> Result<Option<PostId>, PostEditorError> {
        Ok(self
            .service
            .read_working_copy(&self.working_copy_id)?
            .remote_target
            .remote_post
            .map(|remote_post| remote_post.post_id))
    }

    /// What the editor opens with: the working copy, as the latest version of
    /// the post.
    pub fn editor_post(&self) -> Result<EditorPost, PostEditorError> {
        let working_copy = self.service.read_working_copy(&self.working_copy_id)?;
        Ok(EditorPost {
            post_id: working_copy
                .remote_target
                .remote_post
                .map(|remote_post| remote_post.post_id),
            post: serde_json::to_vec(&working_copy.fields.to_rest_fields())
                .expect("A JSON object always serializes"),
        })
    }

    /// Record the post, as the editor has it now, as a version of the working
    /// copy. Call it after every change, whether or not the user saves.
    ///
    /// `post` is the whole post as a JSON object in the REST API's edit shape.
    pub async fn record_change(&self, post: Vec<u8>) -> Result<(), PostEditorError> {
        let _report = self.reports.lock().await;
        let post = json_object(&post)?;
        let mut working_copy = self.service.read_working_copy(&self.working_copy_id)?;
        working_copy.fields.apply_editor_post(&post)?;
        self.record_fields(&working_copy.fields)
    }

    /// Record that the editor sent `request`.
    ///
    /// A request that writes the session's post, creating or updating it,
    /// holds exactly what the editor is sending, so its fields are also
    /// recorded as a version of the working copy: change reports can be a
    /// moment behind.
    pub async fn record_request_sent(
        &self,
        request: SessionRequest,
    ) -> Result<(), PostEditorError> {
        let _report = self.reports.lock().await;
        self.service
            .local_content
            .record_session_request_sent(&self.id, &self.source, &request)?;
        let is_write = matches!(
            classify_request(&request, self.remote_post_id()?),
            Some(PostRequest::Create | PostRequest::Update)
        );
        if let (true, Some(body)) = (is_write, request.body.as_deref()) {
            let mut working_copy = self.service.read_working_copy(&self.working_copy_id)?;
            working_copy
                .fields
                .apply_editor_edits(&json_object(body)?)?;
            self.record_fields(&working_copy.fields)?;
        }
        Ok(())
    }

    /// Record the response the editor got to a request it reported sending.
    ///
    /// If the response is a successful one with the session's whole post in
    /// the edit context, wordpress-rs also takes it as the server's version:
    /// see `take_server_post`. The response is recorded first, so it's kept
    /// even when that fails.
    pub async fn record_request_response(
        &self,
        request_id: String,
        response: SessionResponse,
    ) -> Result<(), PostEditorError> {
        let _report = self.reports.lock().await;
        let record = self.service.local_content.record_session_request_response(
            &self.id,
            &request_id,
            &response,
        )?;
        self.take_server_post(&record, &response)
    }

    /// Record that a request the editor reported sending got no answer, such
    /// as when the device was offline.
    pub async fn record_request_failure(
        &self,
        request_id: String,
        reason: String,
    ) -> Result<(), PostEditorError> {
        let _report = self.reports.lock().await;
        Ok(self.service.local_content.record_session_request_failure(
            &self.id,
            &request_id,
            &reason,
        )?)
    }

    /// The requests the editor reported in this session, in the order it
    /// reported sending them.
    pub fn requests(&self) -> Result<Vec<SessionRequestRecord>, PostEditorError> {
        Ok(self
            .service
            .local_content
            .read_session_requests(self.id.clone())?)
    }

    /// End the session normally, when the editor closes.
    pub fn end(&self) -> Result<(), PostEditorError> {
        Ok(self
            .service
            .local_content
            .end_editing_session(self.id.clone())?)
    }
}

impl PostEditorSession {
    fn record_fields(&self, fields: &PostWorkingCopyFields) -> Result<(), PostEditorError> {
        Ok(self
            .service
            .local_content
            .record_post_working_copy_change_in_session(&self.id, &self.working_copy_id, fields)?)
    }

    /// Take a response the editor got as the server's version of the post.
    ///
    /// Only a successful response for the session's post counts, and only one
    /// whose body is the whole post in the edit context: a read without
    /// `_fields`, or a create or an update, whose responses always are.
    fn take_server_post(
        &self,
        record: &SessionRequestRecord,
        response: &SessionResponse,
    ) -> Result<(), PostEditorError> {
        if !is_success(response.status_code) {
            return Ok(());
        }
        let Some(kind) = classify_request(&record.request, self.remote_post_id()?) else {
            return Ok(());
        };
        let Ok(post) = serde_json::from_slice::<AnyPostWithEditContext>(&response.body) else {
            return Ok(());
        };

        // [decision] The editor's responses update the cache, through the same
        // path wordpress-rs's own responses do, unless the cache already has a
        // newer version. The response itself stays in the request log.
        let cached_is_newer = self
            .service
            .posts
            .read_cached_post(post.id)?
            .is_some_and(|cached| cached.modified_gmt.0 > post.modified_gmt.0);
        if !cached_is_newer {
            self.service.posts.cache_post_from_server(&post)?;
        }

        // [decision] A write the server accepted holds what the editor sent, so
        // its response is the working copy's base version, and a create links
        // the working copy to the new post. A read leaves the base version
        // alone: the working copy's changes still start from the version they
        // were made on.
        if matches!(kind, PostRequest::Create | PostRequest::Update) {
            self.service.local_content.record_post_working_copy_push(
                self.working_copy_id.clone(),
                RemotePostReference::from_remote(&post),
            )?;
        }
        Ok(())
    }
}

// Internal helpers

/// A request for the session's post, told apart by its method and path.
enum PostRequest {
    /// Reads the whole post.
    Read,
    /// Creates the post, for a session whose post isn't on the server yet.
    Create,
    Update,
}

/// What `request` does to the session's post, or `None` if it isn't one of
/// the requests in `PostRequest`, such as a read with `_fields`, or a request
/// for an autosave or another post.
fn classify_request(
    request: &SessionRequest,
    remote_post_id: Option<PostId>,
) -> Option<PostRequest> {
    let (path, query) = match request.path.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (request.path.as_str(), None),
    };
    let segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let segments = match segments.as_slice() {
        ["wp", "v2", "sites", _, rest @ ..] | ["wp", "v2", rest @ ..] => rest,
        _ => return None,
    };
    let method = request.method.to_ascii_uppercase();
    match (segments, method.as_str()) {
        (["posts"], "POST") if remote_post_id.is_none() => Some(PostRequest::Create),
        (["posts", id], method) if id.parse().ok().map(PostId) == remote_post_id => match method {
            "GET" => {
                let partial = query.is_some_and(|query| {
                    url::form_urlencoded::parse(query.as_bytes()).any(|(key, _)| key == "_fields")
                });
                (!partial).then_some(PostRequest::Read)
            }
            "POST" | "PUT" | "PATCH" => Some(PostRequest::Update),
            _ => None,
        },
        _ => None,
    }
}

/// What a request that got no answer means.
fn push_failure(err: RequestExecutionError) -> PostEditorError {
    let reason = err.to_string();
    match err {
        // [decision] Which errors mean the request never reached the server:
        // the device being offline, and no connection to the server. A timeout
        // can come after the server received the request, so it may have
        // been sent, like every other failure.
        RequestExecutionError::RequestExecutionFailed {
            reason:
                RequestExecutionErrorReason::DeviceIsOfflineError { .. }
                | RequestExecutionErrorReason::ConnectionError { .. },
            ..
        } => PostEditorError::NotSent { reason },
        _ => PostEditorError::MaybeSent { reason },
    }
}

/// Header names in lowercase. A header with several values has them joined
/// with `, `.
fn string_headers(headers: &WpNetworkHeaderMap) -> HashMap<String, String> {
    headers
        .to_header_map()
        .iter()
        .fold(HashMap::new(), |mut strings, (name, value)| {
            if let Ok(value) = value.to_str() {
                strings
                    .entry(name.as_str().to_string())
                    .and_modify(|joined: &mut String| {
                        joined.push_str(", ");
                        joined.push_str(value);
                    })
                    .or_insert_with(|| value.to_string());
            }
            strings
        })
}

fn is_success(status_code: u32) -> bool {
    (200..300).contains(&status_code)
}

fn json_object(bytes: &[u8]) -> Result<Map<String, Value>, PostEditorError> {
    match serde_json::from_slice(bytes) {
        Ok(Value::Object(object)) => Ok(object),
        Ok(_) => Err(EditorPostConversionError::NotAnObject.into()),
        Err(err) => Err(PostEditorError::InvalidEditorPost {
            reason: err.to_string(),
        }),
    }
}

fn empty_post_fields() -> PostWorkingCopyFields {
    PostWorkingCopyFields {
        common: WorkingCopyCommonFields {
            title: String::new(),
            content: String::new(),
            excerpt: String::new(),
            slug: None,
            status: StatusSetting::Draft { password: None },
            publish_date: PublishDate::Immediately,
            author: None,
            featured_image: FeaturedImage::None,
            comment_status: None,
            ping_status: None,
            template: None,
            menu_order: None,
        },
        categories: vec![],
        tags: vec![],
        format: wp_api::posts::PostFormat::Standard,
        sticky: false,
        meta: PostMeta::new(),
        additional_fields: WpAdditionalFields::new(),
    }
}

impl From<WpNetworkResponse> for ServerResponse {
    fn from(response: WpNetworkResponse) -> Self {
        let headers = string_headers(&response.response_header_map);
        Self {
            status_code: response.status_code,
            headers,
            body: response.body,
        }
    }
}

#[cfg(test)]
mod tests;
