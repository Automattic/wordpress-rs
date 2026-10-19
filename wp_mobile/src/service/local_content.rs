use std::sync::Arc;
use wp_api::posts::AnyPostWithEditContext;
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

    /// Store a working copy of a new post for `site` and return its ID.
    /// Pushing the working copy creates the post.
    pub fn create_post_working_copy(
        &self,
        working_copy: PostWorkingCopy,
        site: SiteIdentity,
    ) -> Result<WorkingCopyId, LocalContentError> {
        let target = WorkingCopyTarget {
            site,
            remote_post: None,
        };
        self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            let id = WorkingCopyRepository.insert_post(&transaction, &working_copy, &target)?;
            transaction.commit()?;
            Ok(id)
        })
    }

    /// Read the working copy of `post`, a post on `site` fetched with the edit
    /// context, storing a new one made from `post` if it has none yet. Pushing
    /// the working copy updates that post.
    ///
    /// An existing working copy is returned as stored, whatever `post` holds.
    /// Making a new one fails with `RemotePostConversion` if the post can't be
    /// converted, for example because it's trashed.
    pub fn read_or_create_post_working_copy_from_remote(
        &self,
        site: SiteIdentity,
        post: AnyPostWithEditContext,
    ) -> Result<DbPostWorkingCopy, LocalContentError> {
        self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            if let Some(existing) =
                WorkingCopyRepository.select_post_by_remote_post(&transaction, &site, post.id)?
            {
                return Ok(existing);
            }
            let working_copy = PostWorkingCopy::from_remote(&post)?;
            let target = WorkingCopyTarget::for_remote_post(site, &post);
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

    /// Store a working copy of a new page for `site` and return its ID.
    /// Pushing the working copy creates the page.
    pub fn create_page_working_copy(
        &self,
        working_copy: PageWorkingCopy,
        site: SiteIdentity,
    ) -> Result<WorkingCopyId, LocalContentError> {
        let target = WorkingCopyTarget {
            site,
            remote_post: None,
        };
        self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            let id = WorkingCopyRepository.insert_page(&transaction, &working_copy, &target)?;
            transaction.commit()?;
            Ok(id)
        })
    }

    /// Read the working copy of `page`, a page on `site` fetched with the edit
    /// context, storing a new one made from `page` if it has none yet. Pushing
    /// the working copy updates that page.
    ///
    /// An existing working copy is returned as stored, whatever `page` holds.
    /// Making a new one fails with `RemotePostConversion` if the page can't be
    /// converted, for example because it's trashed.
    pub fn read_or_create_page_working_copy_from_remote(
        &self,
        site: SiteIdentity,
        page: AnyPostWithEditContext,
    ) -> Result<DbPageWorkingCopy, LocalContentError> {
        self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            if let Some(existing) =
                WorkingCopyRepository.select_page_by_remote_post(&transaction, &site, page.id)?
            {
                return Ok(existing);
            }
            let working_copy = PageWorkingCopy::from_remote(&page)?;
            let target = WorkingCopyTarget::for_remote_post(site, &page);
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
