use std::sync::Arc;
use wp_api::posts::PostId;
use wp_mobile_local_content::{
    LocalContentError, WpLocalContentStore,
    db_types::{
        editor_session::{
            EditorRequest, EditorRequestRecord, EditorResponse, EditorSession, EditorSessionId,
        },
        working_copy::{
            PageWorkingCopy, PageWorkingCopyFields, PostWorkingCopy, PostWorkingCopyFields,
            WorkingCopyId,
        },
        working_copy_remote_target::{RemotePostReference, SiteIdentity, WorkingCopyRemoteTarget},
        working_copy_version::{WorkingCopyVersion, WorkingCopyVersionId},
    },
    repository::{editor_sessions::EditorSessionRepository, working_copies::WorkingCopyRepository},
};

/// Stores the posts and pages the user creates or edits on the device, as
/// working copies.
///
/// A working copy is the user's version of a post or page. It holds `fields`,
/// the post as the user edits it, and a `remote_target`, the site and post
/// that pushing it updates. Each save that changes its fields adds a version
/// to its history, and its `fields` are the newest version's. A working copy
/// is deleted only when you call one of the delete functions, so call them
/// only when the user asks.
///
/// The service never talks to the server or the API cache. Your app pushes a
/// working copy by sending its fields to the server itself, then records the
/// push with `record_post_working_copy_push` or `record_page_working_copy_push`.
///
/// To edit a post from the server, first look for its working copy with
/// `read_post_working_copy_for_remote_post`. If it has none, create one from
/// the fetched post with `create_post_working_copy`. To write a new post,
/// create a working copy whose remote target has no `remote_post`. Pages work
/// the same way through the page functions.
#[derive(uniffi::Object)]
pub struct LocalContentService {
    store: Arc<WpLocalContentStore>,
}

#[uniffi::export]
impl LocalContentService {
    /// Create the service on `store`. Run the store's migrations first.
    #[uniffi::constructor]
    pub fn new(store: Arc<WpLocalContentStore>) -> Self {
        Self { store }
    }

    /// Read the working copy of post `post_id` on `site`, or `None` if it has
    /// none.
    pub fn read_post_working_copy_for_remote_post(
        &self,
        site: SiteIdentity,
        post_id: PostId,
    ) -> Result<Option<PostWorkingCopy>, LocalContentError> {
        self.store
            .execute(|conn| WorkingCopyRepository.select_post_by_remote_post(conn, &site, post_id))
    }

    /// Create a post working copy, and return it with its new ID.
    ///
    /// For a post from the server, convert the fetched post into `fields` and
    /// into the `RemotePostReference` that `remote_target` holds. For a new
    /// post, leave `remote_target.remote_post` unset.
    ///
    /// Fails with `RemotePostAlreadyHasWorkingCopy` if the remote post already
    /// has a working copy.
    pub fn create_post_working_copy(
        &self,
        remote_target: WorkingCopyRemoteTarget,
        fields: PostWorkingCopyFields,
    ) -> Result<PostWorkingCopy, LocalContentError> {
        self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            let id = WorkingCopyRepository.insert_post(&transaction, &fields, &remote_target)?;
            let created = WorkingCopyRepository
                .select_post(&transaction, &id)?
                .ok_or(LocalContentError::WorkingCopyNotFound { id })?;
            transaction.commit()?;
            Ok(created)
        })
    }

    /// Save the user's changes to a post working copy: `fields` become its
    /// fields, as a new version in its history. Nothing is added if they're
    /// the fields it has.
    ///
    /// Fails with `WorkingCopyNotFound` if no post working copy has the ID.
    pub fn save_post_working_copy(
        &self,
        id: WorkingCopyId,
        fields: PostWorkingCopyFields,
    ) -> Result<(), LocalContentError> {
        let updated = self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            let updated = WorkingCopyRepository.update_post(&transaction, &id, &fields, None)?;
            transaction.commit()?;
            Ok::<_, LocalContentError>(updated)
        })?;
        if updated {
            Ok(())
        } else {
            Err(LocalContentError::WorkingCopyNotFound { id })
        }
    }

    /// Tell the service that your app pushed this post working copy to the
    /// server.
    ///
    /// Call it after every successful push, with a `RemotePostReference` made
    /// from the post the server returned. Its `modified_gmt` becomes the
    /// working copy's base version. For a new post, the post the push created
    /// becomes the working copy's remote post.
    ///
    /// Fails with `WorkingCopyNotFound` if no post working copy has the ID.
    /// Fails with `RemotePostAlreadyHasWorkingCopy` if another working copy
    /// already has the pushed post as its remote post, and with
    /// `PushedToAnotherRemotePost` if this one has a different remote post.
    pub fn record_post_working_copy_push(
        &self,
        id: WorkingCopyId,
        remote_post: RemotePostReference,
    ) -> Result<(), LocalContentError> {
        let recorded = self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            let recorded =
                WorkingCopyRepository.record_post_push(&transaction, &id, remote_post)?;
            transaction.commit()?;
            Ok::<_, LocalContentError>(recorded)
        })?;
        if recorded {
            Ok(())
        } else {
            Err(LocalContentError::WorkingCopyNotFound { id })
        }
    }

    /// Read a post working copy by its ID.
    ///
    /// Returns `None` if no post working copy has the ID.
    pub fn read_post_working_copy(
        &self,
        id: WorkingCopyId,
    ) -> Result<Option<PostWorkingCopy>, LocalContentError> {
        self.store
            .execute(|conn| WorkingCopyRepository.select_post(conn, &id))
    }

    /// Delete a post working copy, discarding the user's changes in it. Call it
    /// only when the user asks to.
    ///
    /// Fails with `WorkingCopyNotFound` if no post working copy has the ID.
    pub fn delete_post_working_copy(&self, id: WorkingCopyId) -> Result<(), LocalContentError> {
        let deleted = self
            .store
            .execute(|conn| WorkingCopyRepository.delete_post(conn, &id))?;
        if deleted {
            Ok(())
        } else {
            Err(LocalContentError::WorkingCopyNotFound { id })
        }
    }

    /// Delete every post and page working copy for `site`, discarding the
    /// user's changes in them, and return how many were deleted.
    ///
    /// Call it only when the user chooses to delete the site's local content,
    /// such as by logging out with "Log out and delete local content".
    /// Removing the site from the API cache doesn't delete its working copies,
    /// so they're still there if the site is added back.
    pub fn delete_working_copies_for_site(
        &self,
        site: SiteIdentity,
    ) -> Result<u64, LocalContentError> {
        self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            let deleted = WorkingCopyRepository.delete_by_site(&transaction, &site)?;
            transaction.commit()?;
            Ok(deleted as u64)
        })
    }

    /// Read every post working copy for `site`, newest `updated_at` first.
    pub fn read_post_working_copies(
        &self,
        site: SiteIdentity,
    ) -> Result<Vec<PostWorkingCopy>, LocalContentError> {
        self.store
            .execute(|conn| WorkingCopyRepository.select_posts_by_site(conn, &site))
    }

    /// Read every post working copy on every site, newest `updated_at` first.
    pub fn read_all_post_working_copies(&self) -> Result<Vec<PostWorkingCopy>, LocalContentError> {
        self.store
            .execute(|conn| WorkingCopyRepository.select_all_posts(conn))
    }

    /// Read the working copy of page `post_id` on `site`, or `None` if it has
    /// none.
    pub fn read_page_working_copy_for_remote_post(
        &self,
        site: SiteIdentity,
        post_id: PostId,
    ) -> Result<Option<PageWorkingCopy>, LocalContentError> {
        self.store
            .execute(|conn| WorkingCopyRepository.select_page_by_remote_post(conn, &site, post_id))
    }

    /// Create a page working copy, and return it with its new ID.
    ///
    /// For a page from the server, convert the fetched page into `fields` and
    /// into the `RemotePostReference` that `remote_target` holds. For a new
    /// page, leave `remote_target.remote_post` unset.
    ///
    /// Fails with `RemotePostAlreadyHasWorkingCopy` if the remote page already
    /// has a working copy.
    pub fn create_page_working_copy(
        &self,
        remote_target: WorkingCopyRemoteTarget,
        fields: PageWorkingCopyFields,
    ) -> Result<PageWorkingCopy, LocalContentError> {
        self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            let id = WorkingCopyRepository.insert_page(&transaction, &fields, &remote_target)?;
            let created = WorkingCopyRepository
                .select_page(&transaction, &id)?
                .ok_or(LocalContentError::WorkingCopyNotFound { id })?;
            transaction.commit()?;
            Ok(created)
        })
    }

    /// Save the user's changes to a page working copy: `fields` become its
    /// fields, as a new version in its history. Nothing is added if they're
    /// the fields it has.
    ///
    /// Fails with `WorkingCopyNotFound` if no page working copy has the ID.
    pub fn save_page_working_copy(
        &self,
        id: WorkingCopyId,
        fields: PageWorkingCopyFields,
    ) -> Result<(), LocalContentError> {
        let updated = self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            let updated = WorkingCopyRepository.update_page(&transaction, &id, &fields, None)?;
            transaction.commit()?;
            Ok::<_, LocalContentError>(updated)
        })?;
        if updated {
            Ok(())
        } else {
            Err(LocalContentError::WorkingCopyNotFound { id })
        }
    }

    /// Tell the service that your app pushed this page working copy to the
    /// server.
    ///
    /// Call it after every successful push, with a `RemotePostReference` made
    /// from the page the server returned. Its `modified_gmt` becomes the
    /// working copy's base version. For a new page, the page the push created
    /// becomes the working copy's remote post.
    ///
    /// Fails with `WorkingCopyNotFound` if no page working copy has the ID.
    /// Fails with `RemotePostAlreadyHasWorkingCopy` if another working copy
    /// already has the pushed page as its remote post, and with
    /// `PushedToAnotherRemotePost` if this one has a different remote post.
    pub fn record_page_working_copy_push(
        &self,
        id: WorkingCopyId,
        remote_post: RemotePostReference,
    ) -> Result<(), LocalContentError> {
        let recorded = self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            let recorded =
                WorkingCopyRepository.record_page_push(&transaction, &id, remote_post)?;
            transaction.commit()?;
            Ok::<_, LocalContentError>(recorded)
        })?;
        if recorded {
            Ok(())
        } else {
            Err(LocalContentError::WorkingCopyNotFound { id })
        }
    }

    /// Read a page working copy by its ID.
    ///
    /// Returns `None` if no page working copy has the ID.
    pub fn read_page_working_copy(
        &self,
        id: WorkingCopyId,
    ) -> Result<Option<PageWorkingCopy>, LocalContentError> {
        self.store
            .execute(|conn| WorkingCopyRepository.select_page(conn, &id))
    }

    /// Delete a page working copy, discarding the user's changes in it. Call it
    /// only when the user asks to.
    ///
    /// Fails with `WorkingCopyNotFound` if no page working copy has the ID.
    pub fn delete_page_working_copy(&self, id: WorkingCopyId) -> Result<(), LocalContentError> {
        let deleted = self
            .store
            .execute(|conn| WorkingCopyRepository.delete_page(conn, &id))?;
        if deleted {
            Ok(())
        } else {
            Err(LocalContentError::WorkingCopyNotFound { id })
        }
    }

    /// Read every page working copy for `site`, newest `updated_at` first.
    pub fn read_page_working_copies(
        &self,
        site: SiteIdentity,
    ) -> Result<Vec<PageWorkingCopy>, LocalContentError> {
        self.store
            .execute(|conn| WorkingCopyRepository.select_pages_by_site(conn, &site))
    }

    /// Read every page working copy on every site, newest `updated_at` first.
    pub fn read_all_page_working_copies(&self) -> Result<Vec<PageWorkingCopy>, LocalContentError> {
        self.store
            .execute(|conn| WorkingCopyRepository.select_all_pages(conn))
    }

    /// Read the history of a post or page working copy: its versions, oldest
    /// first. The newest holds the working copy's fields.
    ///
    /// Fails with `WorkingCopyNotFound` if no working copy has the ID.
    pub fn read_working_copy_versions(
        &self,
        id: WorkingCopyId,
    ) -> Result<Vec<WorkingCopyVersion>, LocalContentError> {
        self.store
            .execute(|conn| WorkingCopyRepository.select_versions(conn, &id))?
            .ok_or(LocalContentError::WorkingCopyNotFound { id })
    }

    /// Read the fields of one version of a post working copy, or `None` if no
    /// version of a post working copy has the ID.
    pub fn read_post_working_copy_version(
        &self,
        id: WorkingCopyVersionId,
    ) -> Result<Option<PostWorkingCopyFields>, LocalContentError> {
        self.store
            .execute(|conn| WorkingCopyRepository.select_post_version(conn, &id))
    }

    /// Read the fields of one version of a page working copy, or `None` if no
    /// version of a page working copy has the ID.
    pub fn read_page_working_copy_version(
        &self,
        id: WorkingCopyVersionId,
    ) -> Result<Option<PageWorkingCopyFields>, LocalContentError> {
        self.store
            .execute(|conn| WorkingCopyRepository.select_page_version(conn, &id))
    }

    /// Start an editor session for a working copy, and return it.
    ///
    /// `editor_identity` is who the editor makes its requests as, in a form
    /// that isn't a credential, such as a hash of the token. It's recorded as
    /// given.
    ///
    /// Fails with `WorkingCopyNotFound` if no working copy has the ID.
    pub fn create_editor_session(
        &self,
        working_copy_id: WorkingCopyId,
        editor_identity: Option<String>,
    ) -> Result<EditorSession, LocalContentError> {
        self.store.execute(|conn| {
            let id = EditorSessionRepository.insert(
                conn,
                &working_copy_id,
                editor_identity.as_deref(),
            )?;
            EditorSessionRepository
                .select(conn, &id)?
                .ok_or(LocalContentError::EditorSessionNotFound { id })
        })
    }

    /// End a session: when its editor closes, or once a session that didn't
    /// end normally has been dealt with. Ending a session that has already
    /// ended keeps its first `ended_at`.
    ///
    /// Fails with `EditorSessionNotFound` if no session has the ID.
    pub fn end_editor_session(&self, id: EditorSessionId) -> Result<(), LocalContentError> {
        let ended = self
            .store
            .execute(|conn| EditorSessionRepository.end(conn, &id))?;
        found_session(ended, id)
    }

    /// Read every session that hasn't ended, oldest first.
    ///
    /// That includes sessions an editor still has open.
    pub fn read_unfinished_editor_sessions(&self) -> Result<Vec<EditorSession>, LocalContentError> {
        self.store
            .execute(|conn| EditorSessionRepository.select_unfinished(conn))
    }

    /// Read the requests the editor reported during a session, in the order
    /// it reported sending them.
    pub fn read_editor_session_requests(
        &self,
        id: EditorSessionId,
    ) -> Result<Vec<EditorRequestRecord>, LocalContentError> {
        self.store
            .execute(|conn| EditorSessionRepository.select_requests(conn, &id))
    }
}

impl LocalContentService {
    /// Read a session by its ID, or `None` if no session has it.
    pub fn read_editor_session(
        &self,
        id: &EditorSessionId,
    ) -> Result<Option<EditorSession>, LocalContentError> {
        self.store
            .execute(|conn| EditorSessionRepository.select(conn, id))
    }

    /// Record a change the editor made in a session: `fields` become the post
    /// working copy's fields, as a version recorded in the session, and the
    /// session's `last_recorded_at` becomes now.
    ///
    /// Fails with `WorkingCopyNotFound` if no post working copy has
    /// `working_copy_id`, and with `EditorSessionNotFound` if no session has
    /// `session_id`.
    pub fn record_post_working_copy_change_in_session(
        &self,
        session_id: &EditorSessionId,
        working_copy_id: &WorkingCopyId,
        fields: &PostWorkingCopyFields,
    ) -> Result<(), LocalContentError> {
        self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            if !WorkingCopyRepository.update_post(
                &transaction,
                working_copy_id,
                fields,
                Some(session_id),
            )? {
                return Err(LocalContentError::WorkingCopyNotFound {
                    id: working_copy_id.clone(),
                });
            }
            found_session(
                EditorSessionRepository.record_change(&transaction, session_id)?,
                session_id.clone(),
            )?;
            transaction.commit()?;
            Ok(())
        })
    }

    /// Record that the editor sent `request` during a session.
    pub fn record_editor_request_sent(
        &self,
        id: &EditorSessionId,
        request: &EditorRequest,
    ) -> Result<(), LocalContentError> {
        self.store
            .execute(|conn| EditorSessionRepository.record_request_sent(conn, id, request))
    }

    /// Record the response to a request the session recorded as sent, and
    /// return the completed record.
    pub fn record_editor_request_response(
        &self,
        id: &EditorSessionId,
        request_id: &str,
        response: &EditorResponse,
    ) -> Result<EditorRequestRecord, LocalContentError> {
        self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            EditorSessionRepository.record_request_response(
                &transaction,
                id,
                request_id,
                response,
            )?;
            let record = EditorSessionRepository
                .select_request(&transaction, id, request_id)?
                .ok_or_else(|| LocalContentError::EditorRequestNotFound {
                    request_id: request_id.to_string(),
                })?;
            transaction.commit()?;
            Ok(record)
        })
    }

    /// Record that a request the session recorded as sent got no answer.
    pub fn record_editor_request_failure(
        &self,
        id: &EditorSessionId,
        request_id: &str,
        reason: &str,
    ) -> Result<(), LocalContentError> {
        self.store.execute(|conn| {
            EditorSessionRepository.record_request_failure(conn, id, request_id, reason)
        })
    }
}

fn found_session(found: bool, id: EditorSessionId) -> Result<(), LocalContentError> {
    if found {
        Ok(())
    } else {
        Err(LocalContentError::EditorSessionNotFound { id })
    }
}
