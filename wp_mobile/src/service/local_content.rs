use std::sync::Arc;
use wp_mobile_local_content::{
    LocalContentError, WpLocalContentStore,
    db_types::{
        working_copy::{
            DbPageWorkingCopy, DbPostWorkingCopy, PageWorkingCopy, PostWorkingCopy, WorkingCopyId,
        },
        working_copy_target::{SiteIdentity, WorkingCopyTarget},
    },
    repository::working_copies::WorkingCopyRepository,
};

/// Service for content created or edited on the device.
///
/// Reads and writes go to the local content database only; nothing here
/// touches the network or the API cache.
#[derive(uniffi::Object)]
pub struct LocalContentService {
    store: Arc<WpLocalContentStore>,
}

#[uniffi::export]
impl LocalContentService {
    #[uniffi::constructor]
    pub fn new(store: Arc<WpLocalContentStore>) -> Self {
        Self { store }
    }

    /// Read the working copy for `target`, storing `working_copy` as a new one
    /// if there's none yet.
    ///
    /// With a remote post in `target`, an existing working copy of that post
    /// is returned as stored, whatever `working_copy` holds, and pushing the
    /// working copy updates that post. Without one, `working_copy` is always
    /// stored as a new post, which pushing the working copy creates.
    pub fn read_or_create_post_working_copy(
        &self,
        target: WorkingCopyTarget,
        working_copy: PostWorkingCopy,
    ) -> Result<DbPostWorkingCopy, LocalContentError> {
        self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            if let Some(remote_post) = target.remote_post
                && let Some(existing) = WorkingCopyRepository.select_post_by_remote_post(
                    &transaction,
                    &target.site,
                    remote_post.post_id,
                )?
            {
                return Ok(existing);
            }
            let id = WorkingCopyRepository.insert_post(&transaction, &working_copy, &target)?;
            let created = WorkingCopyRepository
                .select_post(&transaction, &id)?
                .ok_or(LocalContentError::WorkingCopyNotFound { id })?;
            transaction.commit()?;
            Ok(created)
        })
    }

    /// Save new fields for an existing post working copy. Its target is unchanged.
    ///
    /// Fails with `WorkingCopyNotFound` if no post working copy has the given ID.
    pub fn save_post_working_copy(
        &self,
        id: WorkingCopyId,
        working_copy: PostWorkingCopy,
    ) -> Result<(), LocalContentError> {
        let updated = self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            let updated = WorkingCopyRepository.update_post(&transaction, &id, &working_copy)?;
            transaction.commit()?;
            Ok::<_, LocalContentError>(updated)
        })?;
        if updated {
            Ok(())
        } else {
            Err(LocalContentError::WorkingCopyNotFound { id })
        }
    }

    /// Read a post working copy and its target from the local content database.
    ///
    /// Returns `None` if no post working copy has the given ID.
    pub fn read_post_working_copy(
        &self,
        id: WorkingCopyId,
    ) -> Result<Option<DbPostWorkingCopy>, LocalContentError> {
        self.store
            .execute(|conn| WorkingCopyRepository.select_post(conn, &id))
    }

    /// Delete a post working copy, discarding the user's changes in it. Call it
    /// only when the user asks to: user content is never lost otherwise.
    ///
    /// Fails with `WorkingCopyNotFound` if no post working copy has the given ID.
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

    /// Read every post working copy targeting `site`, most recently updated first.
    pub fn read_post_working_copies(
        &self,
        site: SiteIdentity,
    ) -> Result<Vec<DbPostWorkingCopy>, LocalContentError> {
        self.store
            .execute(|conn| WorkingCopyRepository.select_posts_by_site(conn, &site))
    }

    /// Read every post working copy, whatever site it targets, most recently
    /// updated first.
    pub fn read_all_post_working_copies(
        &self,
    ) -> Result<Vec<DbPostWorkingCopy>, LocalContentError> {
        self.store
            .execute(|conn| WorkingCopyRepository.select_all_posts(conn))
    }

    /// Read the working copy for `target`, storing `working_copy` as a new one
    /// if there's none yet.
    ///
    /// With a remote page in `target`, an existing working copy of that page
    /// is returned as stored, whatever `working_copy` holds, and pushing the
    /// working copy updates that page. Without one, `working_copy` is always
    /// stored as a new page, which pushing the working copy creates.
    pub fn read_or_create_page_working_copy(
        &self,
        target: WorkingCopyTarget,
        working_copy: PageWorkingCopy,
    ) -> Result<DbPageWorkingCopy, LocalContentError> {
        self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            if let Some(remote_post) = target.remote_post
                && let Some(existing) = WorkingCopyRepository.select_page_by_remote_post(
                    &transaction,
                    &target.site,
                    remote_post.post_id,
                )?
            {
                return Ok(existing);
            }
            let id = WorkingCopyRepository.insert_page(&transaction, &working_copy, &target)?;
            let created = WorkingCopyRepository
                .select_page(&transaction, &id)?
                .ok_or(LocalContentError::WorkingCopyNotFound { id })?;
            transaction.commit()?;
            Ok(created)
        })
    }

    /// Save new fields for an existing page working copy. Its target is unchanged.
    ///
    /// Fails with `WorkingCopyNotFound` if no page working copy has the given ID.
    pub fn save_page_working_copy(
        &self,
        id: WorkingCopyId,
        working_copy: PageWorkingCopy,
    ) -> Result<(), LocalContentError> {
        let updated = self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            let updated = WorkingCopyRepository.update_page(&transaction, &id, &working_copy)?;
            transaction.commit()?;
            Ok::<_, LocalContentError>(updated)
        })?;
        if updated {
            Ok(())
        } else {
            Err(LocalContentError::WorkingCopyNotFound { id })
        }
    }

    /// Read a page working copy and its target from the local content database.
    ///
    /// Returns `None` if no page working copy has the given ID.
    pub fn read_page_working_copy(
        &self,
        id: WorkingCopyId,
    ) -> Result<Option<DbPageWorkingCopy>, LocalContentError> {
        self.store
            .execute(|conn| WorkingCopyRepository.select_page(conn, &id))
    }

    /// Delete a page working copy, discarding the user's changes in it. Call it
    /// only when the user asks to: user content is never lost otherwise.
    ///
    /// Fails with `WorkingCopyNotFound` if no page working copy has the given ID.
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

    /// Read every page working copy targeting `site`, most recently updated first.
    pub fn read_page_working_copies(
        &self,
        site: SiteIdentity,
    ) -> Result<Vec<DbPageWorkingCopy>, LocalContentError> {
        self.store
            .execute(|conn| WorkingCopyRepository.select_pages_by_site(conn, &site))
    }

    /// Read every page working copy, whatever site it targets, most recently
    /// updated first.
    pub fn read_all_page_working_copies(
        &self,
    ) -> Result<Vec<DbPageWorkingCopy>, LocalContentError> {
        self.store
            .execute(|conn| WorkingCopyRepository.select_all_pages(conn))
    }
}
