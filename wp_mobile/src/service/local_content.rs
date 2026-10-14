use std::sync::Arc;
use wp_mobile_local_content::{
    LocalContentError, WpLocalContentStore,
    db_types::{
        working_copy::{
            DbPageWorkingCopy, DbPostWorkingCopy, PageWorkingCopy, PostWorkingCopy, WorkingCopyId,
        },
        working_copy_target::WorkingCopyTarget,
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

    /// Store a new post working copy with its target and return its ID.
    pub fn create_post_working_copy(
        &self,
        working_copy: PostWorkingCopy,
        target: WorkingCopyTarget,
    ) -> Result<WorkingCopyId, LocalContentError> {
        self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            let id = WorkingCopyRepository.insert_post(&transaction, &working_copy, &target)?;
            transaction.commit()?;
            Ok(id)
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

    /// Store a new page working copy with its target and return its ID.
    pub fn create_page_working_copy(
        &self,
        working_copy: PageWorkingCopy,
        target: WorkingCopyTarget,
    ) -> Result<WorkingCopyId, LocalContentError> {
        self.store.execute(|conn| {
            let transaction = conn.transaction()?;
            let id = WorkingCopyRepository.insert_page(&transaction, &working_copy, &target)?;
            transaction.commit()?;
            Ok(id)
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
}
