# Proposal: the editor saves through wordpress-rs

## Summary

While the editor is open, it tells the app about every change to the post, and wordpress-rs records each one in the post's working copy, so nothing the user writes depends on the network.

When the user saves, GutenbergKit runs Gutenberg's own `savePost()`, so plugins' save hooks run. The one network request inside it, which sends the post to the server, goes to the app instead of straight to the server. The app hands it to wordpress-rs, which:

1. sends the post to the server, creating it there if it doesn't exist yet;
2. records the server's answer in its cache and in the working copy;
3. hands the server's response back to the editor.

Everything about the post goes through wordpress-rs: reading it, storing it and sending it. wordpress-rs also records each editor session, so a session that ended without finishing, and a request that never got an answer, are found the next time the app runs. An app that doesn't give GutenbergKit a request handler gets GutenbergKit's built-in behavior, unchanged.

## Why

Two pieces of work need to agree on who saves a post.

**GutenbergKit needs the editor to run the save.** Plugins can run code when a post is saved, through hooks on Gutenberg's [`savePost()`](https://github.com/WordPress/gutenberg/blob/%40wordpress/editor%4016.1.0/packages/editor/src/store/actions.js#L243-L476). Today the app saves the post itself, so `savePost()` never runs, and neither do those hooks ([CMM-2024](https://linear.app/a8c/issue/CMM-2024/videopress-metadata-change-not-persisted) is one resulting bug, in VideoPress). [Editor-driven saving](https://linear.app/a8c/document/editor-driven-saving-design-5520af8f36b9) fixes this by having the editor save the post to the server while it's open, and the app after it closes.

**wordpress-rs needs to own the post.** Local content storage ([wordpress-rs#1682](https://github.com/Automattic/wordpress-rs/pull/1682), a draft) keeps the posts the user edits on the device, so they're never lost, and finds conflicts by comparing them with what wordpress-rs last fetched from the server. That comparison is only right if wordpress-rs fetches and sends the post itself. Its rules are in [Local Content Principles](https://github.com/Automattic/wordpress-rs/blob/local-content-storage/Documentation/local-content-principles.md).

Editor-driven saving as designed has two writers for the same post: the editor and the app, each with its own copy of it and its own recovery rules. This proposal keeps one. The editor still runs `savePost()`, so plugins' hooks run, and wordpress-rs sends the request inside it.

## Terms

- **Working copy**: the user's version of a post or page, stored on the device by wordpress-rs. Only the user changes it, and only an explicit user action deletes it.
- **Cache**: wordpress-rs's copy of what the server returned the last time wordpress-rs fetched the post. It's a separate database from the working copies.
- **Record a change**: store the post, as the editor has it, in its working copy. It happens after every change, without the user asking.
- **Push**: sending a working copy to the server. wordpress-rs decides whether a push creates the post or updates it.
- **Editor session**: one stretch of editing a post, from opening the editor to closing it. wordpress-rs records each session in its database: when it started, when a change was last recorded, the push it's waiting on an answer for, and whether it ended normally.
- **Which version is the source of truth**: while the editor is open, the editor is the source of truth: it has the latest version of the post, and the working copy follows it a moment behind, since recording a change is a database write in the background. Once the editor closes, the working copy is the source of truth. Another editor open on the same post elsewhere, such as WP Admin, doesn't change this: what it writes reaches wordpress-rs as a change on the server, and it's a conflict if the working copy changed too.
- **Base version**: the server's `modified_gmt` for the version of the post a working copy's edits start from. Each successful push replaces it with the `modified_gmt` in the server's response.
- **Conflict**: the working copy and the server both changed since the base version. wordpress-rs finds it by comparing the working copy with the cache field by field; it's never stored as a flag.

## The rule, and what it covers

**Everything about the post goes through wordpress-rs.** Nothing else fetches the post, stores it or sends it. wordpress-rs can only compare the working copy with the server correctly if its cache is the best picture of the server available. That's only true if wordpress-rs makes every request for the post itself. If the editor fetched or saved the post itself, wordpress-rs would compare against an older copy, and could report conflicts that aren't there or miss ones that are.

It also makes editing durable. wordpress-rs records every change, every editor session and every push still waiting for an answer in its database. Those records survive the app crashing, the system ending the app in the background, and the connection dropping. What the editor holds lives in its web page, and is gone if the page's process ends. After any of these, wordpress-rs still knows what the user wrote, and which push it was waiting on, from its own records.

It covers the post endpoint: the post being edited (`/wp/v2/posts/{id}`, or the endpoint for its post type) and the requests under its path, such as its autosaves. It doesn't cover the other things the editor reads or writes, such as categories and tags created from the editor, site settings, menus, template parts, and plugins' own endpoints. Those stay as they are today. Any exception inside the post endpoint is called out below as a `[decision]`.

## The layers

| | Layer | Depends on |
| -- | -- | -- |
| A | GutenbergKit, the editor library | Nothing from wordpress-rs. It defines an interface the app can implement. |
| B | The app: GutenbergKit's example app first, then WordPress-iOS and WordPress-Android | Both GutenbergKit and wordpress-rs. It's the only layer that knows both. |
| C | `wp_mobile`, wordpress-rs's business layer | The API apps call. Decides how posts are recorded, pushed and compared. |
| D | `wp_mobile_local_content`, in wordpress-rs | Stores the working copies and the editor sessions. |

GutenbergKit never depends on wordpress-rs, and wordpress-rs never depends on GutenbergKit.

## How to read the pseudocode

Names are placeholders. Two markers:

- `[decision]`: a choice made in exactly one place, which can change without touching the other layers.
- `[not designed]`: a path nobody has designed yet. The pseudocode shows where it plugs in.

The main path comes first, layer by layer: the user edits a post, and then saves it successfully. The other paths follow: new posts, pushing without a connection, lost responses, a session that didn't end normally, pushing after the editor closes, and autosaves, previews and revisions.

## The main path: editing a post and saving it

### A. GutenbergKit

GutenbergKit gains three things the app can use:

1. **Change reports.** Whenever anything in the post changes, the editor tells the app. This includes the content, settings such as the categories, and plugins' fields. It happens whether or not the user saves. The app then reads the post from the editor and records the change.
2. **A request handler.** The editor sends its network requests for the post to the app, instead of to the server. These include the request that reads the post, and the one that sends it to the server when the user saves.
3. **Setting the post's ID.** Once a new post has been created on the server, the app tells the editor its ID.

`[decision]` What a change report carries, to agree with GutenbergKit. It can be only the fact that something changed, with the app reading the whole post afterwards, as in the sketch below. It can also carry the changes, or the whole post. Any of these works for wordpress-rs. Today GutenbergKit reports only that the content changed, and the app can read only the title and the content (`getTitleAndContent()`). This proposal needs every field, including the settings and plugins' fields.

The interface, sketched in Kotlin. The Swift version would mirror it.

```kotlin
// 1. Change reports.
interface EditorPostChangeListener {
    /** Anything in the post changed. Called whether or not the user saves. */
    fun onPostChanged()
}

// The post as the editor has it now: every field, in the REST API's edit shape.
suspend fun GutenbergView.currentPost(): ByteArray   // JSON

// 2. The request handler.
interface EditorPostRequestHandler {
    /**
     * Send a request for the post being edited. Return the server's response as
     * it arrived, error statuses included. Throw only when the server couldn't be
     * reached.
     */
    suspend fun perform(request: EditorPostRequest): EditorPostResponse
}

data class EditorPostRequest(
    val method: String,
    val path: String,
    val body: ByteArray?,              // JSON
)

data class EditorPostResponse(
    val status: Int,
    val headers: Map<String, String>,
    val body: ByteArray,               // the server's bytes, unchanged
)

// 3. Setting the post's ID.
fun GutenbergView.setPostId(id: Long)

// Optional, like `mediaUploader` today: set them before the editor loads.
gutenbergView.postChangeListener = listener
gutenbergView.postRequestHandler = handler
```

How GutenbergKit builds these is up to GutenbergKit. The rest of this section covers the parts of Gutenberg and GutenbergKit the proposal relies on.

**How Gutenberg saves a post.** This proposal doesn't change it. When the user saves, Gutenberg runs three steps:

1. The [`editor.preSavePost`](https://github.com/WordPress/gutenberg/blob/%40wordpress/editor%4016.1.0/packages/editor/src/store/actions.js#L279) filter runs: plugins can add or change fields.
2. [`saveEntityRecord`](https://github.com/WordPress/gutenberg/blob/%40wordpress/editor%4016.1.0/packages/editor/src/store/actions.js#L291) sends one `apiFetch` request: `PUT /wp/v2/posts/{id}` with the fields that changed.
3. The [`editor.savePost`](https://github.com/WordPress/gutenberg/blob/%40wordpress/editor%4016.1.0/packages/editor/src/store/actions.js#L331) action runs: plugins do their post-save work. It's skipped if step 2 failed.

The proposal changes only where step 2's request goes. `apiFetch` is the function Gutenberg sends its REST API requests with. GutenbergKit adds middleware to it ([`src/utils/api-fetch.js`](https://github.com/wordpress-mobile/GutenbergKit/blob/c776c07c64acbd9cec72dcd8ac9fdf819944238b/src/utils/api-fetch.js#L45-L105)). Middleware are functions that see each request before it leaves the page, and can change it or answer it themselves. One of them, [`blockPostWritesMiddleware`](https://github.com/wordpress-mobile/GutenbergKit/blob/c776c07c64acbd9cec72dcd8ac9fdf819944238b/src/utils/api-fetch.js#L267-L313), rejects every request that writes to the post being edited, because today the app saves the post itself. The proposal replaces it with middleware that sends the post's requests to the app's handler:

```js
// Every apiFetch request passes through this before it leaves the page.
async function postRequestMiddleware( options, next ) {
	// Only requests for the post being edited, e.g. /wp/v2/posts/123 or under it.
	if ( ! isPostPath( options.path ) ) {
		return next( options );
	}
	// No handler from the app: the request goes to the server as it does today.
	if ( ! getGBKit().hasPostRequestHandler ) {
		return next( options );
	}

	// Rejects only when the app couldn't reach the server, like a failed fetch.
	const response = await bridge.performPostRequest( {
		method: options.method,
		path: options.path,
		body: options.data, // only the changed fields, plus what plugins added
	} );

	// The server's response, parsed as if the editor had sent the request itself.
	return parseLikeApiFetch( response, options );
}
```

`bridge.performPostRequest` sends the request to the app and waits for its answer. iOS already supports this kind of round trip: the editor uses one to ask the app for the latest content ([`requestLatestContent`](https://github.com/wordpress-mobile/GutenbergKit/blob/c776c07c64acbd9cec72dcd8ac9fdf819944238b/ios/Sources/GutenbergKit/Sources/EditorViewController.swift#L313), a script message handler with a reply). Android needs an asynchronous reply, which editor-driven saving also plans.

The app returns the server's response exactly as the server sent it. Gutenberg takes that response as the post's saved version. If wordpress-rs rebuilt it from its own types, it could leave out fields wordpress-rs doesn't model, and the editor would show them as unsaved.

**Opening the editor** needs no change in GutenbergKit. It builds the post from what the app passes in when it creates the editor, and doesn't fetch the post itself ([`src/utils/prime-post-entity.js`](https://github.com/wordpress-mobile/GutenbergKit/blob/c776c07c64acbd9cec72dcd8ac9fdf819944238b/src/utils/prime-post-entity.js#L10-L32)). With this proposal, the app passes in what wordpress-rs returns from `editor_post()`.

GutenbergKit takes that post as the post's saved version. Gutenberg counts as unsaved only the edits made in the editor after that ([`isEditedPostDirty`](https://github.com/WordPress/gutenberg/blob/%40wordpress/editor%4016.1.0/packages/editor/src/store/selectors.js#L94-L107)), so changes in the working copy that were never pushed don't show as unsaved. That's how GutenbergKit works today, with the content the app passes in, and this proposal doesn't change it. The next save still sends them, since wordpress-rs pushes what differs from the server, whatever the editor shows.

### B. The app

The app connects GutenbergKit to wordpress-rs. It implements GutenbergKit's interfaces by calling the editor session in wordpress-rs, and converts requests and responses between the two libraries' types. The rest is decided in `wp_mobile`, so every app behaves the same: how changes are recorded, whether to fetch before pushing, which fields to send, and how to record the result. The one exception is when to push after the editor has closed (see "Pushing after the editor closes").

```kotlin
// Opening a post starts an editor session.
val session = postEditorService.openPost(site = site, postId = postId)
val gutenbergView = GutenbergView(
    configuration = configurationFor(session.editorPost()),   // what the editor shows
    dependencies = dependencies,
    coroutineScope = scope,
    context = context,
)

// Every change is recorded in the working copy, in the background.
gutenbergView.postChangeListener = EditorPostChangeListener {
    scope.launch { session.recordChange(gutenbergView.currentPost()) }
}

// Every request for the post goes through the session.
gutenbergView.postRequestHandler = WordPressRsPostRequestHandler(session)

// Closing the editor ends the session.
override fun onEditorClosed() {
    scope.launch { session.end() }
}

class WordPressRsPostRequestHandler(
    private val session: PostEditorSession,
) : EditorPostRequestHandler {
    override suspend fun perform(request: EditorPostRequest): EditorPostResponse {
        val response = session.performEditorRequest(
            method = request.method,
            path = request.path,
            body = request.body,
        )
        return EditorPostResponse(response.status, response.headers, response.body)
    }
}
```

### C. `wp_mobile`

`wp_mobile` already has [`LocalContentService`](https://github.com/Automattic/wordpress-rs/blob/e6d2a8d85d06427d31a2b4f9e2468c4811c037be/wp_mobile/src/service/local_content.rs#L15-L35), which stores working copies and never talks to the server. The proposal adds `PostEditorService` beside it, which brings together the working copies, the editor sessions, the cache and the network. Apps use it to open a post in the editor, and to push. Opening a post returns a `PostEditorSession`, which records the editor's changes and answers the editor's requests.

```rust
/// Editing posts and pages: the editor's changes and requests go through here.
pub struct PostEditorService {
    local_content: Arc<LocalContentService>,
    cache: Arc<WpApiCache>,
    api_client: Arc<WpApiClient>,
}

impl PostEditorService {
    /// Start an editor session for a post from the server. Uses the post's
    /// working copy if it has one, otherwise creates one from the post in the
    /// cache, fetching it if needed.
    pub async fn open_post(&self, site: SiteIdentity, post_id: PostId) -> Result<PostEditorSession>;

    /// Start an editor session for a new post: a working copy with no remote post.
    pub fn open_new_post(&self, site: SiteIdentity) -> Result<PostEditorSession>;

    /// Send a working copy to the server, creating the post there if it doesn't
    /// exist yet. Saves the user makes in the editor come here, and so do the app's pushes after
    /// the editor closes. There is one way to push.
    pub async fn push_working_copy(&self, id: WorkingCopyId) -> Result<RawResponse, PushError> {
        // [decision] One push at a time per working copy: a push waits for one
        // already in flight.
        let _in_flight = self.push_lock(id).await;
        let working_copy = self.local_content.read_post_working_copy(id)?;

        let Some(remote_post) = working_copy.remote_target.remote_post else {
            return self.create_remote_post(&working_copy).await; // see "New posts"
        };

        // [decision] Fetch the post first, to check for a conflict.
        // [decision] Send every field, or only the changed ones.
        let params = push_params(&working_copy);
        // [decision] Record the attempt before sending. See "Lost responses".

        match self.api_client.update_post_raw(remote_post.post_id, params).await {
            // The server answered. Whatever the status, even an error, the response goes back as it is.
            Ok(response) => {
                if response.is_success() {
                    let post = response.parse_post()?;
                    // wordpress-rs's own request, so it fills the cache.
                    self.cache.upsert_post(&post)?;
                    // The base version comes from this response, not a later
                    // fetch, which could already include someone else's change.
                    self.local_content.record_post_working_copy_push(id, RemotePostReference::from(&post))?;
                }
                Ok(response.raw())
            }
            // No answer. See "Pushing without a connection".
            Err(error) => Err(self.handle_push_failure(&working_copy, error)),
        }
    }
}

impl PostEditorSession {
    /// What the editor shows when it opens: the cached post with the working
    /// copy laid over it.
    pub fn editor_post(&self) -> EditorPost;

    /// Record the post, as the editor has it now, in the working copy. Called
    /// after every change, in the background.
    pub async fn record_change(&self, post: EditorPost) -> Result<()> {
        let mut working_copy = self.local_content.read_post_working_copy(self.working_copy_id)?;
        working_copy.fields.apply_editor_post(post)?;
        self.local_content.save_post_working_copy(working_copy.id, working_copy.fields.clone())?;
        self.local_content.record_editor_session_change(self.session_id)?;

        // [decision] Create a new post on the server as soon as it has
        // something in it. See "New posts".
        Ok(())
    }

    /// A request from the editor for this post, told apart by method and path.
    pub async fn perform_editor_request(&self, request: EditorRequest) -> Result<RawResponse, PushError> {
        match request.kind() {
            // A write means the user saved.
            Write { edits } => self.record_and_push(edits).await,
            Read => self.fetch(request).await,
            // See "Autosaves, previews and revisions".
            Autosave | Revisions => not_designed(request),
        }
    }

    async fn record_and_push(&self, edits: EditorEdits) -> Result<RawResponse, PushError> {
        // Changes are recorded in the background, so the working copy can be a
        // moment behind the editor. The write holds exactly what the editor is
        // sending, so it's recorded first.
        let mut working_copy = self.local_content.read_post_working_copy(self.working_copy_id)?;
        working_copy.fields.apply_editor_edits(edits)?;
        self.local_content.save_post_working_copy(working_copy.id, working_copy.fields.clone())?;

        self.service.push_working_copy(working_copy.id).await
    }

    /// The editor reading the post during the session: fetch, fill the cache,
    /// and answer with the server's response as it is. While the editor is
    /// open, it's the source of truth.
    async fn fetch(&self, request: EditorRequest) -> Result<RawResponse, PushError>;

    /// End the session normally, when the editor closes.
    pub async fn end(&self) -> Result<()> {
        self.local_content.end_editor_session(self.session_id)
    }
}
```

### D. `wp_mobile_local_content`

What exists, in [wordpress-rs#1682](https://github.com/Automattic/wordpress-rs/pull/1682): the [working copy types](https://github.com/Automattic/wordpress-rs/blob/e6d2a8d85d06427d31a2b4f9e2468c4811c037be/wp_mobile_local_content/src/db_types/working_copy.rs#L132) and their database. Through `LocalContentService`, apps can create, read, save and delete working copies, and record a push.

The proposal adds two things.

**Room for the fields a working copy doesn't model yet.** Plugins can add their own fields to a post, as post meta or as top-level fields of their own, and the editor holds them along with the rest of the post. The cache already stores both for posts, as [`meta`](https://github.com/Automattic/wordpress-rs/blob/2d099fc911acd5d06b3526dba0b3db7d1e22776a/wp_mobile_cache/migrations/0002-create-posts-table.sql#L36) and [`additional_fields`](https://github.com/Automattic/wordpress-rs/blob/2d099fc911acd5d06b3526dba0b3db7d1e22776a/wp_mobile_cache/migrations/0011-add-additional-fields-to-posts-tables.sql#L1). The working copy stores them the same way, as JSON, and pushes them as they are.

```rust
pub struct PostWorkingCopyFields {
    // ... the existing typed fields: title, content, status, categories, ...
    /// Post meta from the editor, one value per meta key.
    pub meta: Map<String, Json>,
    /// Fields from the editor that have no typed field, such as fields plugins
    /// add, one value per key.
    pub additional_fields: Map<String, Json>,
}

impl PostWorkingCopyFields {
    /// Take the post as the editor has it. Typed fields are converted from
    /// their REST shape (`date`, `status`, ...) to the working copy's
    /// (`PublishDate`, `StatusSetting`, ...). Other keys go into `meta` and
    /// `additional_fields`, one value per key.
    pub fn apply_editor_post(&mut self, post: EditorPost) -> Result<()>;

    /// Apply the fields an editor write sends: the same conversion, for only the
    /// fields that changed. Other keys are merged key by key, so earlier edits
    /// stay.
    pub fn apply_editor_edits(&mut self, edits: EditorEdits) -> Result<()>;
}
```

These conversions are the reverse of the existing one from a fetched post to working copy fields ([`wp_mobile_local_content/src/remote.rs`](https://github.com/Automattic/wordpress-rs/blob/e6d2a8d85d06427d31a2b4f9e2468c4811c037be/wp_mobile_local_content/src/remote.rs)). Recording a change uses the existing [`LocalContentService::save_post_working_copy`](https://github.com/Automattic/wordpress-rs/blob/e6d2a8d85d06427d31a2b4f9e2468c4811c037be/wp_mobile/src/service/local_content.rs#L80-L100).

Comparing these fields works key by key, as for the typed fields. wordpress-rs can say which key conflicts, but not what the value means, so resolving a conflict in one is a choice between the user's value and the server's.

**Editor sessions.** One record per session, stored with the working copies rather than in the cache, so clearing the cache never removes it.

```rust
pub struct EditorSession {
    pub id: EditorSessionId,
    pub working_copy_id: WorkingCopyId,
    pub started_at: WpGmtDateTime,
    /// When a change was last recorded in the working copy.
    pub last_recorded_at: Option<WpGmtDateTime>,
    /// Set when the session ends normally. A session without it didn't.
    pub ended_at: Option<WpGmtDateTime>,
    /// The push sent without an answer yet. See "Lost responses".
    pub unconfirmed_push: Option<PushAttempt>,
}

pub fn create_editor_session(&self, working_copy_id: WorkingCopyId) -> Result<EditorSession>;
pub fn record_editor_session_change(&self, id: EditorSessionId) -> Result<()>;
pub fn end_editor_session(&self, id: EditorSessionId) -> Result<()>;
pub fn read_unfinished_editor_sessions(&self) -> Result<Vec<EditorSession>>;
```

## The other paths

### New posts

Opening a new post creates its working copy and its editor session straight away. Until the post exists on the server, the editor uses the placeholder ID `-1`. The ID matters to the code running inside the editor, not to wordpress-rs, which knows the post from the session:

- Gutenberg stores the post under its ID. If a save of post `-1` came back as post 123, Gutenberg would keep them as two posts, and `-1` would still look unsaved.
- Plugins' save hooks get the post's ID, and need a real one to attach their changes to.
- Requests outside the post endpoint carry the ID to the server. Media uploads, for example, attach the file to the post by its ID.

So a new post gets its server ID before the editor's first `savePost()`. wordpress-rs creates the post from the working copy, which already holds what the user wrote, since every change is recorded in it. Then the app gives the editor the ID.

```kotlin
// B. The app, once the session has the post's server ID.
session.onRemotePostCreated { postId ->
    gutenbergView.setPostId(postId)   // the editor switches to the real ID
}
```

`[decision]` When wordpress-rs creates the post:

- **As soon as it has something in it.** `record_change` pushes the working copy after the first change that leaves it non-empty. The user's first tap on Save is then an ordinary update. A post the user abandons stays on the site as a draft.
- **When the user first taps Save.** The app asks the session to create the post, gives the editor the ID, and then calls the editor's `savePost()`. Nothing reaches the server before the user asks for it.

A save can also start inside the editor, from a keyboard shortcut or a plugin. If the post has no ID yet, the editor refuses it, as editor-driven saving plans, and the app runs the same steps.

Inside Gutenberg, the post's ID is the key it stores the post under, so switching it means moving the post to the new key with its unsaved edits. Editor-driven saving plans this as `adoptPost` ([CMM-2564](https://linear.app/a8c/issue/CMM-2564/let-the-host-hand-a-newly-created-post-id-to-the-editor)), which a spike showed works without reloading the editor ([CMM-2558](https://linear.app/a8c/issue/CMM-2558/spike-switch-the-editor-to-a-new-post-id-without-remounting)). It clears the undo history ([CMM-2742](https://linear.app/a8c/issue/CMM-2742/clear-the-undo-history-when-the-editor-adopts-a-new-post-id)).

```rust
// C. Reached from `push_working_copy` when the working copy has no remote post.
async fn create_remote_post(&self, working_copy: &PostWorkingCopy) -> Result<RawResponse, PushError> {
    // [decision] Record the attempt before sending. A create whose response is
    // lost may still have made the post, and there's no ID to look it up by.
    let params = create_params(working_copy);

    match self.api_client.create_post_raw(params).await {
        Ok(response) => {
            if response.is_success() {
                let post = response.parse_post()?;
                self.cache.upsert_post(&post)?;
                // Links the working copy to the post the create made. The
                // session tells the app the new ID.
                self.local_content.record_post_working_copy_push(working_copy.id, RemotePostReference::from(&post))?;
            }
            Ok(response.raw())
        }
        Err(error) => Err(self.handle_push_failure(working_copy, error)),
    }
}
```

A create is never retried automatically: if its response may have been lost, sending it again can make a second post. WordPress-Android does this today ([CMM-2740](https://linear.app/a8c/issue/CMM-2740/wordpress-android-duplicate-drafts-created-after-failed-post-preview)). `[decision]` How the app finds a post that a lost create may have made. WordPress's REST API has no idempotency key, so the server can't recognize a retried create.

### Pushing without a connection

Every change is recorded in the working copy without the network, so a failed push loses nothing. What's left to decide is what the failure means.

```rust
// C. What a push that got no answer means.
fn handle_push_failure(&self, working_copy: &PostWorkingCopy, error: WpApiError) -> PushError {
    match error.reason() {
        // [decision] Which errors mean the request never reached the server.
        // `wp_api` reports, among others, `DeviceIsOfflineError`,
        // `ConnectionError` and `HttpTimeoutError`. A timeout can come after
        // the server received the request.
        reason if is_never_sent(reason) => PushError::NotSent,
        // The server may have applied it. See "Lost responses".
        _ => PushError::MaybeSent,
    }
}
```

`[decision]` Whether to check the connection before sending. wordpress-rs has no connectivity check today; this sketch sends the request and classifies the error.

In the editor, the app's handler throws, so the request fails the way a failed `fetch` would. Gutenberg's `savePost()` stops there, and the `editor.savePost` action doesn't run.

### Lost responses

A push that may have reached the server stays unconfirmed until wordpress-rs fetches the post again. The attempt is recorded on the editor session, so it survives the app being closed.

```rust
// C. Run the next time wordpress-rs fetches the post: when the editor opens,
// when the app returns to the foreground, before the next push.
async fn settle_unconfirmed_push(&self, session: &EditorSession) -> Result<()> {
    // [decision] Only possible if the attempt was recorded before sending: the
    // fields sent and the base version they started from.
    let Some(attempt) = &session.unconfirmed_push else {
        // Nothing recorded. wordpress-rs can't tell its own push from someone
        // else's change, so comparing reports a conflict.
        return Ok(());
    };
    let server_post = self.fetch_post(attempt.remote_post_id).await?; // fills the cache

    if attempt.matches(&server_post) {
        // The push landed.
        self.local_content.record_post_working_copy_push(session.working_copy_id, RemotePostReference::from(&server_post))?;
    }
    // Otherwise the working copy is compared with the cache as usual: either
    // the push didn't land and it can be sent again, or someone else changed
    // the post, which is a conflict.
    self.local_content.clear_unconfirmed_push(session.id)
}
```

Editor-driven saving needs the same record of attempts, for the same reason. With this proposal it's built once, on the editor session in `wp_mobile_local_content`.

### A session that didn't end normally

A session that ends normally records when it ended. One that doesn't, because the app crashed or the system ended it in the background, is still unfinished the next time the app runs. The record can't say which of the two happened.

```rust
// C. When the app starts.
pub async fn recover_unfinished_sessions(&self) -> Result<Vec<UnfinishedSession>> {
    let sessions = self.local_content.read_unfinished_editor_sessions()?;
    for session in &sessions {
        // A push it was waiting on is settled first. See "Lost responses".
        self.settle_unconfirmed_push(session).await?;
    }
    // [decision] What the app does with each: reopen the editor, offer to,
    // or leave the working copy for the post list.
    Ok(sessions.into_iter().map(UnfinishedSession::from).collect())
}
```

Nothing the user wrote is lost either way: every change was recorded in the working copy, up to `last_recorded_at`.

### Pushing after the editor closes

The app calls `PostEditorService::push_working_copy` itself, the same function the user's saves in the editor use.

```kotlin
// B. [decision] When the app pushes a working copy that wasn't pushed while the editor was open:
// coming back online, returning to the foreground, the user tapping Save.
postEditorService.pushWorkingCopy(workingCopyId)
```

While the editor is open, only its session pushes the post: the editor is the source of truth, and the working copy can be a moment behind it.

Plugins' save hooks don't run for pushes made after the editor closes, because the editor isn't involved. This is a deliberate exception. Business logic belongs in wordpress-rs, and what happens around an editor save should only concern the editor. Plugins are third-party code that can't move into wordpress-rs, so their hooks run only for saves made while the editor is open. The same goes for a new post that wordpress-rs creates as soon as it has content: the create runs no hooks, and they first run when the user saves.

### Autosaves, previews and revisions

`[not designed]` Autosaves, previews and revisions aren't designed yet. Their requests are below the post's path (`/wp/v2/posts/{id}/autosaves` and `/wp/v2/posts/{id}/revisions`), so the middleware in A hands them to the app, and `perform_editor_request` in C is where they plug in. A preview needs the post's content on the server, because the server renders the preview.

## Not verified yet

The pseudocode is written against the code of GutenbergKit and wordpress-rs, but these points haven't been tried. A proof of concept in GutenbergKit's example app comes next, and anything it contradicts comes back to this proposal.

- That answering the editor's save with the server's response, through the app, leaves Gutenberg in the same state as when the editor sends the request itself.
- Whether Gutenberg sends all of a post's meta in a save, or only the meta keys that changed.
- That a WordPress update response always holds the whole post in the edit context, whichever fields the request sent.
