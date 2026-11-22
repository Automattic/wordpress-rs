//! Editing posts in a block editor, with wordpress-rs reading, storing and
//! pushing the post.
//!
//! While the editor is open, it reports each change to the app, which records
//! it in the post's working copy through a `PostEditorSession`. The editor's
//! requests for the post go through the session too: a write records the
//! fields it sends and pushes the working copy, and the server's response
//! goes back to the editor as it arrived.
//!
//! The choices still open in the design are marked `[decision]`, each in one
//! place.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use futures::lock::Mutex as AsyncMutex;
use serde_json::{Map, Value};
use wp_api::{
    WpAdditionalFields,
    api_client::{IsWpApiClientDelegate, WpApiClient},
    api_error::{RequestExecutionError, RequestExecutionErrorReason},
    date::WpGmtDateTime,
    middleware::PerformsRequests,
    posts::{AnyPostWithEditContext, PostId, PostMeta, PostRetrieveParams},
    request::{
        InnerRequestBuilder, WpNetworkRequest, WpNetworkResponse,
        endpoint::{ApiEndpointUrl, ApiUrlResolver, posts_endpoint::PostEndpointType},
    },
};
use wp_mobile_local_content::{
    LocalContentError,
    db_types::{
        editor_session::{EditorSession, EditorSessionId},
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

/// A request the editor sends for the post it's editing.
#[derive(Debug, Clone, uniffi::Record)]
pub struct EditorRequest {
    /// The HTTP method, such as `GET`, `POST` or `PUT`.
    pub method: String,
    /// The REST API path with its query, such as `/wp/v2/posts/42?context=edit`.
    pub path: String,
    /// The JSON body, if the request has one.
    pub body: Option<Vec<u8>>,
}

/// What the editor opens with: the version of the post it takes as saved,
/// and the working copy's changes to it as unsaved edits.
///
/// Keeping the working copy's changes out of the saved version means that
/// when the editor later takes in another copy of the post from the server,
/// they stay: only the saved version under them changes.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EditorPost {
    /// The post as wordpress-rs last fetched it from the server, a JSON object
    /// in the REST API's edit shape. `None` for a post that isn't on the
    /// server yet, for which the editor starts from an empty post.
    pub saved_post: Option<Vec<u8>>,
    /// The working copy's fields that differ from `saved_post`, or from an
    /// empty post if there's none, as a JSON object in the shape the REST API
    /// takes them in a write.
    pub edits: Vec<u8>,
}

/// What wordpress-rs found when the editor reported that its saved version of
/// the post changed.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SavedPostCheck {
    /// The editor's saved version is the server's version wordpress-rs has.
    InSync,
    /// wordpress-rs has a newer version from the server. Give it to the editor
    /// as its saved version.
    Newer { saved_post: Vec<u8> },
    /// The editor's saved version has the same modified date as the server's
    /// version wordpress-rs has, but differs from it in `fields`. Only
    /// something other than the server changes it that way, so it's recorded
    /// on the session, and the editor's version isn't taken as the server's.
    Mismatch { fields: Vec<String> },
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
    #[error("{method} {path} isn't a request for a post")]
    NotThePostEndpoint { method: String, path: String },
    #[error("{path} is for another post than the session's")]
    AnotherPost { path: String },
    #[error("The post isn't on the server yet")]
    NotOnServer,
    /// The post changed on the server since the working copy's base version,
    /// and the working copy differs from it in `differing_fields`. Which side
    /// changed each field isn't known, since the cache holds only the server's
    /// latest version. Nothing was sent.
    #[error("The post changed on the server, and differs in {differing_fields:?}")]
    Conflict { differing_fields: Vec<String> },
    /// Autosaves, previews, revisions and trashing aren't designed yet.
    #[error("{method} {path} isn't designed yet")]
    NotDesigned { method: String, path: String },
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

/// Editing a site's posts: the editor's changes and requests go through here.
///
/// Open a post with `open_post` or `open_new_post`, which start an editor
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
    /// The sessions a `PostEditorSession` in this process has open.
    open_sessions: Mutex<HashSet<EditorSessionId>>,
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

    /// Start an editor session for post `post_id` from the server.
    ///
    /// Uses the post's working copy if it has one, and otherwise creates one
    /// from the post in the cache. Fetches the post if the cache doesn't have
    /// it.
    pub async fn open_post(
        self: Arc<Self>,
        post_id: PostId,
    ) -> Result<Arc<PostEditorSession>, PostEditorError> {
        let cached = match self.posts.read_cached_post(post_id)? {
            Some(post) => post,
            None => self.fetch_post(post_id).await?,
        };
        let working_copy = match self
            .local_content
            .read_post_working_copy_for_remote_post(self.site.clone(), post_id)?
        {
            Some(working_copy) => working_copy,
            None => self.local_content.create_post_working_copy(
                WorkingCopyRemoteTarget {
                    site: self.site.clone(),
                    remote_post: Some(RemotePostReference::from_remote(&cached)),
                },
                PostWorkingCopyFields::from_remote(&cached)?,
            )?,
        };
        self.start_session(working_copy.id)
    }

    /// Start an editor session for a new post: a working copy with no remote
    /// post.
    pub fn open_new_post(self: Arc<Self>) -> Result<Arc<PostEditorSession>, PostEditorError> {
        let working_copy = self.local_content.create_post_working_copy(
            WorkingCopyRemoteTarget {
                site: self.site.clone(),
                remote_post: None,
            },
            empty_post_fields(),
        )?;
        self.start_session(working_copy.id)
    }

    /// Send a working copy to the server, creating the post there if it
    /// doesn't exist yet, and return the server's response.
    ///
    /// Use it after the editor has closed. While it's open, push through its
    /// session, which also records the push on the session.
    ///
    /// Updating a post fetches it first, then sends only the fields that
    /// differ from the server's. Fails with `Conflict` if the post changed on
    /// the server since the base version and the working copy differs from
    /// it, and with `NotSent` or `MaybeSent` if no answer came. Any answer,
    /// error statuses included, is returned. On a successful one, the post
    /// goes into the cache and becomes the working copy's base version.
    pub async fn push_working_copy(
        &self,
        working_copy_id: WorkingCopyId,
    ) -> Result<ServerResponse, PostEditorError> {
        self.push(&working_copy_id, None).await
    }

    /// The editor sessions on this site that didn't end normally, such as
    /// when the app crashed or the system ended it, oldest first.
    ///
    /// A push such a session was waiting on is settled first, if it updated
    /// a post: the post is fetched, and if it holds what the push sent, the
    /// push is recorded. Sessions open in this process aren't included.
    pub async fn recover_unfinished_sessions(&self) -> Result<Vec<EditorSession>, PostEditorError> {
        let open = self
            .open_sessions
            .lock()
            .expect("The open sessions lock isn't poisoned")
            .clone();
        let unfinished: Vec<EditorSession> = self
            .local_content
            .read_unfinished_editor_sessions()?
            .into_iter()
            .filter(|session| !open.contains(&session.id))
            .collect();

        let mut recovered = Vec::new();
        for session in unfinished {
            let on_this_site = self
                .local_content
                .read_post_working_copy(session.working_copy_id.clone())?
                .is_some_and(|working_copy| working_copy.remote_target.site == self.site);
            if !on_this_site {
                continue;
            }
            if let Err(err) = self.settle_unconfirmed_push(&session).await {
                log::warn!(
                    "Settling the push of editor session {} failed: {err}",
                    session.id.0
                );
            }
            recovered.push(
                self.local_content
                    .read_editor_session(&session.id)?
                    .unwrap_or(session),
            );
        }
        Ok(recovered)
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
            open_sessions: Mutex::new(HashSet::new()),
            push_locks: Mutex::new(HashMap::new()),
        }
    }

    fn start_session(
        self: Arc<Self>,
        working_copy_id: WorkingCopyId,
    ) -> Result<Arc<PostEditorSession>, PostEditorError> {
        let session = self
            .local_content
            .create_editor_session(working_copy_id.clone())?;
        self.open_sessions
            .lock()
            .expect("The open sessions lock isn't poisoned")
            .insert(session.id.clone());
        Ok(Arc::new(PostEditorSession {
            service: self,
            id: session.id,
            working_copy_id,
        }))
    }

    fn read_working_copy(&self, id: &WorkingCopyId) -> Result<PostWorkingCopy, PostEditorError> {
        self.local_content
            .read_post_working_copy(id.clone())?
            .ok_or_else(|| PostEditorError::WorkingCopyNotFound { id: id.clone() })
    }

    /// Fetch a post with the edit context and put it in the cache.
    async fn fetch_post(&self, post_id: PostId) -> Result<AnyPostWithEditContext, PostEditorError> {
        let post = self
            .api_client
            .posts()
            .retrieve_with_edit_context(
                &PostEndpointType::Posts,
                &post_id,
                &PostRetrieveParams::default(),
            )
            .await
            .map_err(|err| PostEditorError::FetchFailed {
                reason: err.to_string(),
            })?
            .data;
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

    /// Push a working copy, recording the attempt on `session_id` if it's
    /// given, and return the server's response.
    async fn push(
        &self,
        working_copy_id: &WorkingCopyId,
        session_id: Option<&EditorSessionId>,
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
                let (remote_post, changes) = self
                    .changes_to_push(working_copy_id, remote_post, session_id)
                    .await?;
                (Some(remote_post), changes)
            }
        };
        let request = self.write_request(remote_post.map(|remote_post| remote_post.post_id), &body);
        let body = serde_json::to_string(&body).expect("A JSON object always serializes");

        // [decision] Record the attempt before sending, so a push whose
        // answer is lost is found later. Only a push through a session is
        // recorded, since the record lives on the session.
        if let Some(session_id) = session_id {
            self.local_content
                .record_unconfirmed_push(session_id, remote_post, &body)?;
        }

        let result = self
            .api_client
            .posts()
            .perform(Arc::new(request), None)
            .await;
        let response = match result {
            Ok(response) => response,
            Err(err) => {
                let err = push_failure(err);
                if let (PostEditorError::NotSent { .. }, Some(session_id)) = (&err, session_id) {
                    self.local_content.clear_unconfirmed_push(session_id)?;
                }
                return Err(err);
            }
        };
        // The server answered, whatever the status, so nothing is unconfirmed.
        if let Some(session_id) = session_id {
            self.local_content.clear_unconfirmed_push(session_id)?;
        }
        if !is_success(&response) {
            return Ok(ServerResponse::from(response));
        }

        let post: AnyPostWithEditContext =
            serde_json::from_slice(&response.body).map_err(|err| {
                PostEditorError::UnreadablePushResponse {
                    status_code: response.status_code,
                    reason: err.to_string(),
                }
            })?;
        // wordpress-rs's own request, so it fills the cache.
        self.posts.cache_post_from_server(&post)?;
        // The base version comes from this response, not a later fetch, which
        // could already include someone else's change.
        self.local_content.record_post_working_copy_push(
            working_copy_id.clone(),
            RemotePostReference::from_remote(&post),
        )?;
        Ok(ServerResponse::from(response))
    }

    /// Fetch the post a working copy updates, and return its remote post
    /// and the fields that differ from the server's.
    ///
    /// The fetch settles a push the session is waiting on first, so a push
    /// whose answer was lost isn't mistaken for someone else's change. Fails
    /// with `Conflict` if the post changed on the server since the base
    /// version and the working copy differs from it, and with `NotSent` if
    /// the fetch failed.
    async fn changes_to_push(
        &self,
        working_copy_id: &WorkingCopyId,
        remote_post: RemotePostReference,
        session_id: Option<&EditorSessionId>,
    ) -> Result<(RemotePostReference, Map<String, Value>), PostEditorError> {
        // [decision] Fetch the post first, to check for a conflict and to send
        // only what changed.
        let server_post =
            self.fetch_post(remote_post.post_id)
                .await
                .map_err(|err| PostEditorError::NotSent {
                    reason: format!("Fetching the post before pushing failed: {err}"),
                })?;
        if let Some(session) = session_id
            .map(|session_id| self.local_content.read_editor_session(session_id))
            .transpose()?
            .flatten()
        {
            self.settle_with(&session, &server_post)?;
        }

        // The base version may have moved while settling.
        let working_copy = self.read_working_copy(working_copy_id)?;
        let remote_post = working_copy
            .remote_target
            .remote_post
            .unwrap_or(remote_post);
        let server_fields = PostWorkingCopyFields::from_remote(&server_post)?;
        // [decision] Send only the fields that differ from the server's.
        let changes = working_copy.fields.rest_fields_changed_from(&server_fields);

        if server_post.modified_gmt != remote_post.base_modified_gmt {
            if changes.is_empty() {
                // The server already has what the user has: nothing conflicts.
                let remote_post = RemotePostReference::from_remote(&server_post);
                self.local_content
                    .record_post_working_copy_push(working_copy_id.clone(), remote_post)?;
                return Ok((remote_post, changes));
            }
            // [not designed] What happens to a conflict. The working copy keeps
            // everything, and nothing is sent.
            return Err(PostEditorError::Conflict {
                differing_fields: changes.into_iter().map(|(key, _)| key).collect(),
            });
        }
        Ok((remote_post, changes))
    }

    /// Find out whether the push a session was waiting on landed, and record
    /// it if it did. The unconfirmed push is cleared either way, and the
    /// working copy is compared with the server as usual.
    async fn settle_unconfirmed_push(
        &self,
        session: &EditorSession,
    ) -> Result<(), PostEditorError> {
        // [decision] How to find a post a lost create may have made: not
        // designed. The unconfirmed create stays on the session for the app to
        // see.
        let Some(remote_post) = session
            .unconfirmed_push
            .as_ref()
            .and_then(|push| push.remote_post)
        else {
            return Ok(());
        };
        let server_post = self.fetch_post(remote_post.post_id).await?;
        self.settle_with(session, &server_post)
    }

    /// Settle the session's unconfirmed update with `server_post`, the post as
    /// just fetched: the push landed if the post holds every field it sent.
    fn settle_with(
        &self,
        session: &EditorSession,
        server_post: &AnyPostWithEditContext,
    ) -> Result<(), PostEditorError> {
        let Some(push) = &session.unconfirmed_push else {
            return Ok(());
        };
        if push.remote_post.is_none() {
            return Ok(());
        }
        let sent: Map<String, Value> =
            serde_json::from_str(&push.body).map_err(|err| PostEditorError::LocalContent {
                reason: format!("The unconfirmed push's body isn't a JSON object: {err}"),
            })?;
        if push_landed(&sent, &PostWorkingCopyFields::from_remote(server_post)?) {
            self.local_content.record_post_working_copy_push(
                session.working_copy_id.clone(),
                RemotePostReference::from_remote(server_post),
            )?;
        }
        self.local_content.clear_unconfirmed_push(&session.id)?;
        Ok(())
    }

    /// Send the editor's read of the post as it is, with the session's
    /// authentication. A successful response fills the cache, if it's a post
    /// with the edit context.
    async fn fetch_for_editor(
        &self,
        post_id: PostId,
        query: Option<&str>,
    ) -> Result<WpNetworkResponse, PostEditorError> {
        let mut url = self
            .api_url_resolver
            .resolve(
                "wp/v2".to_string(),
                vec!["posts".to_string(), post_id.0.to_string()],
            )
            .inner
            .clone();
        if let Some(query) = query {
            url.query_pairs_mut()
                .extend_pairs(url::form_urlencoded::parse(query.as_bytes()));
        }
        let auth_provider = self.api_client.posts().get_delegate().auth_provider.clone();
        let request: WpNetworkRequest =
            InnerRequestBuilder::new(auth_provider).get(ApiEndpointUrl::new(url));
        let response = self
            .api_client
            .posts()
            .perform(Arc::new(request), None)
            .await
            .map_err(push_failure)?;
        if is_success(&response)
            && let Ok(post) = serde_json::from_slice::<AnyPostWithEditContext>(&response.body)
        {
            self.posts.cache_post_from_server(&post)?;
        }
        Ok(response)
    }
}

/// One editor session: a post open in the editor, from opening it to closing
/// it.
///
/// While the editor is open, the editor is the source of truth: record each
/// change with `record_change`, and send the editor's requests for the post
/// through `perform_editor_request`. Call `end` when the editor closes.
#[derive(uniffi::Object)]
pub struct PostEditorSession {
    service: Arc<PostEditorService>,
    id: EditorSessionId,
    working_copy_id: WorkingCopyId,
}

impl Drop for PostEditorSession {
    fn drop(&mut self) {
        if let Ok(mut open_sessions) = self.service.open_sessions.lock() {
            open_sessions.remove(&self.id);
        }
    }
}

#[uniffi::export]
impl PostEditorSession {
    pub fn id(&self) -> EditorSessionId {
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

    /// What the editor opens with: the cached post as its saved version, and
    /// the working copy's changes to it as edits.
    pub fn editor_post(&self) -> Result<EditorPost, PostEditorError> {
        let working_copy = self.service.read_working_copy(&self.working_copy_id)?;
        let cached = match working_copy.remote_target.remote_post {
            Some(remote_post) => self.service.posts.read_cached_post(remote_post.post_id)?,
            None => None,
        };
        let (saved_post, base) = match &cached {
            Some(cached) => (
                Some(serde_json::to_vec(cached).expect("A post always serializes")),
                PostWorkingCopyFields::from_remote(cached)?,
            ),
            None => (None, empty_post_fields()),
        };
        let edits = working_copy.fields.rest_fields_changed_from(&base);
        Ok(EditorPost {
            saved_post,
            edits: serde_json::to_vec(&edits).expect("A JSON object always serializes"),
        })
    }

    /// Check the editor's saved version of the post against the server's
    /// version wordpress-rs has, after the editor reported that it changed.
    ///
    /// `change` is a JSON object: `modified_gmt`, the saved version's modified
    /// date, and `fields`, the fields that changed in it, in the REST API's
    /// edit shape. The editor's version is never taken as the server's: if
    /// it's newer than the cached one, wordpress-rs fetches the post.
    pub async fn record_saved_post_change(
        &self,
        change: Vec<u8>,
    ) -> Result<SavedPostCheck, PostEditorError> {
        let Some(post_id) = self.remote_post_id()? else {
            return Ok(SavedPostCheck::InSync);
        };
        let change = json_object(&change)?;
        let invalid = |reason: &str| PostEditorError::InvalidEditorPost {
            reason: format!("The saved post change {reason}"),
        };
        let editor_modified: WpGmtDateTime = change
            .get("modified_gmt")
            .cloned()
            .and_then(|modified| serde_json::from_value(modified).ok())
            .ok_or_else(|| invalid("has no valid `modified_gmt`"))?;
        let changed_fields = match change.get("fields") {
            Some(Value::Object(fields)) => fields.clone(),
            None => Map::new(),
            Some(_) => return Err(invalid("has `fields` that isn't an object")),
        };

        let mut server = match self.service.posts.read_cached_post(post_id)? {
            Some(cached) if cached.modified_gmt.0 >= editor_modified.0 => cached,
            // The editor took in a newer version than wordpress-rs has, or
            // wordpress-rs has none.
            _ => self.service.fetch_post(post_id).await?,
        };
        if server.modified_gmt.0 < editor_modified.0 {
            // Still older after fetching: the server doesn't have the
            // editor's version, so it didn't come from the server.
            server = self.service.fetch_post(post_id).await?;
        }
        if server.modified_gmt.0 > editor_modified.0 {
            return Ok(SavedPostCheck::Newer {
                saved_post: serde_json::to_vec(&server).expect("A post always serializes"),
            });
        }

        let server_fields = PostWorkingCopyFields::from_remote(&server)?;
        let mut editor_fields = server_fields.clone();
        editor_fields.apply_editor_edits(&changed_fields)?;
        let differing: Vec<String> = editor_fields
            .rest_fields_changed_from(&server_fields)
            .into_iter()
            .map(|(key, _)| key)
            .collect();
        if differing.is_empty() {
            return Ok(SavedPostCheck::InSync);
        }
        self.service
            .local_content
            .record_base_mismatch(&self.id, &differing)?;
        Ok(SavedPostCheck::Mismatch { fields: differing })
    }

    /// Record the post, as the editor has it now, in the working copy. Call it
    /// after every change, whether or not the user saves.
    ///
    /// `post` is the whole post as a JSON object in the REST API's edit shape.
    pub fn record_change(&self, post: Vec<u8>) -> Result<(), PostEditorError> {
        let post = json_object(&post)?;
        let mut working_copy = self.service.read_working_copy(&self.working_copy_id)?;
        working_copy.fields.apply_editor_post(&post)?;
        self.service
            .local_content
            .save_post_working_copy(working_copy.id, working_copy.fields)?;
        self.service
            .local_content
            .record_editor_session_change(self.id.clone())?;
        // [decision] When a new post is created on the server: on the first
        // save, through `push`, never here. Nothing reaches the server before
        // the user asks for it.
        Ok(())
    }

    /// Answer a request from the editor for this post.
    ///
    /// A write (`POST`, `PUT` or `PATCH`) means the user saved: the fields it
    /// sends are recorded in the working copy, and the working copy is
    /// pushed. A read (`GET`) is sent to the server as it is, and a successful
    /// one fills the cache. Either way, the server's response comes back as it
    /// arrived: while the editor is open, it's the source of truth, and the
    /// working copy follows it.
    ///
    /// A write fails as `PostEditorService::push_working_copy` does. Fails with
    /// `NotDesigned` for autosaves, revisions and trashing.
    pub async fn perform_editor_request(
        &self,
        request: EditorRequest,
    ) -> Result<ServerResponse, PostEditorError> {
        let remote_post_id = self.remote_post_id()?;
        match classify_editor_request(&request, remote_post_id)? {
            EditorRequestKind::Write => self.record_and_push(request.body.as_deref()).await,
            EditorRequestKind::Read { query } => {
                let post_id = remote_post_id.ok_or(PostEditorError::NotOnServer)?;
                self.service
                    .fetch_for_editor(post_id, query.as_deref())
                    .await
                    .map(ServerResponse::from)
            }
            EditorRequestKind::NotDesigned => Err(PostEditorError::NotDesigned {
                method: request.method,
                path: request.path,
            }),
        }
    }

    /// Push the working copy, creating the post on the server if it isn't
    /// there yet, and return the server's response. For a new post, call it
    /// when the user first saves, then give the editor `remote_post_id`.
    pub async fn push(&self) -> Result<ServerResponse, PostEditorError> {
        self.service
            .push(&self.working_copy_id, Some(&self.id))
            .await
    }

    /// End the session normally, when the editor closes.
    pub fn end(&self) -> Result<(), PostEditorError> {
        Ok(self
            .service
            .local_content
            .end_editor_session(self.id.clone())?)
    }
}

impl PostEditorSession {
    async fn record_and_push(
        &self,
        body: Option<&[u8]>,
    ) -> Result<ServerResponse, PostEditorError> {
        // Changes are recorded in the background, so the working copy can be a
        // moment behind the editor. The write holds exactly what the editor is
        // sending, so it's recorded first.
        let edits = body.map(json_object).transpose()?.unwrap_or_default();
        let mut working_copy = self.service.read_working_copy(&self.working_copy_id)?;
        working_copy.fields.apply_editor_edits(&edits)?;
        self.service
            .local_content
            .save_post_working_copy(working_copy.id, working_copy.fields)?;
        self.service
            .local_content
            .record_editor_session_change(self.id.clone())?;
        self.push().await
    }
}

// Internal helpers

enum EditorRequestKind {
    Read { query: Option<String> },
    Write,
    NotDesigned,
}

/// The placeholder ID the editor uses for a post that isn't on the server yet.
const PLACEHOLDER_POST_ID: i64 = -1;

/// Tell the editor's request apart by its method and path, and check that it's
/// for the session's post.
fn classify_editor_request(
    request: &EditorRequest,
    remote_post_id: Option<PostId>,
) -> Result<EditorRequestKind, PostEditorError> {
    let not_the_post_endpoint = || PostEditorError::NotThePostEndpoint {
        method: request.method.clone(),
        path: request.path.clone(),
    };
    let (path, query) = match request.path.split_once('?') {
        Some((path, query)) => (path, Some(query.to_string())),
        None => (request.path.as_str(), None),
    };
    let segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let segments = match segments.as_slice() {
        ["wp", "v2", "sites", _, rest @ ..] | ["wp", "v2", rest @ ..] => rest,
        _ => return Err(not_the_post_endpoint()),
    };
    let method = request.method.to_ascii_uppercase();
    let is_write = matches!(method.as_str(), "POST" | "PUT" | "PATCH");

    match segments {
        ["posts"] if method == "POST" => Ok(EditorRequestKind::Write),
        ["posts", id, rest @ ..] => {
            let id: i64 = id.parse().map_err(|_| not_the_post_endpoint())?;
            if id != PLACEHOLDER_POST_ID && Some(PostId(id)) != remote_post_id {
                return Err(PostEditorError::AnotherPost {
                    path: request.path.clone(),
                });
            }
            match (rest, method.as_str()) {
                // Autosaves, previews and revisions are below the post's path.
                ([_, ..], _) | ([], "DELETE") => Ok(EditorRequestKind::NotDesigned),
                ([], "GET") => Ok(EditorRequestKind::Read { query }),
                ([], _) if is_write => Ok(EditorRequestKind::Write),
                _ => Err(not_the_post_endpoint()),
            }
        }
        _ => Err(not_the_post_endpoint()),
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

/// Whether the server's post holds every field `sent` sent: for `meta`, every
/// meta key it sent.
fn push_landed(sent: &Map<String, Value>, server: &PostWorkingCopyFields) -> bool {
    let server = server.to_rest_fields();
    sent.iter().all(|(key, value)| match (key.as_str(), value) {
        ("meta", Value::Object(meta)) => meta.iter().all(|(meta_key, meta_value)| {
            server
                .get("meta")
                .and_then(|server_meta| server_meta.get(meta_key))
                == Some(meta_value)
        }),
        _ => server.get(key) == Some(value),
    })
}

fn is_success(response: &WpNetworkResponse) -> bool {
    (200..300).contains(&response.status_code)
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
        let headers = response.response_header_map.to_header_map().iter().fold(
            HashMap::<String, String>::new(),
            |mut headers, (name, value)| {
                if let Ok(value) = value.to_str() {
                    headers
                        .entry(name.as_str().to_string())
                        .and_modify(|joined| {
                            joined.push_str(", ");
                            joined.push_str(value);
                        })
                        .or_insert_with(|| value.to_string());
                }
                headers
            },
        );
        Self {
            status_code: response.status_code,
            headers,
            body: response.body,
        }
    }
}

#[cfg(test)]
mod tests;
