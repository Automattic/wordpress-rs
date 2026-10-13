use std::sync::Arc;
use wp_mobile_local_content::{
    LocalContentError, WpLocalContentStore,
    db_types::post_working_copy::{DbPostWorkingCopy, PostWorkingCopy, PostWorkingCopyId},
    repository::post_working_copies::PostWorkingCopyRepository,
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

    /// Store a new post working copy and return its ID.
    pub fn create_post_working_copy(
        &self,
        working_copy: PostWorkingCopy,
    ) -> Result<PostWorkingCopyId, LocalContentError> {
        self.store
            .execute(|conn| PostWorkingCopyRepository.insert(conn, &working_copy))
    }

    /// Save new fields for an existing post working copy.
    ///
    /// Fails with `PostWorkingCopyNotFound` if no working copy has the given ID.
    pub fn save_post_working_copy(
        &self,
        id: PostWorkingCopyId,
        working_copy: PostWorkingCopy,
    ) -> Result<(), LocalContentError> {
        let updated = self
            .store
            .execute(|conn| PostWorkingCopyRepository.update(conn, &id, &working_copy))?;
        if updated {
            Ok(())
        } else {
            Err(LocalContentError::PostWorkingCopyNotFound { id })
        }
    }

    /// Read a post working copy from the local content database.
    ///
    /// Returns `None` if no working copy has the given ID.
    pub fn read_post_working_copy(
        &self,
        id: PostWorkingCopyId,
    ) -> Result<Option<DbPostWorkingCopy>, LocalContentError> {
        self.store
            .execute(|conn| PostWorkingCopyRepository.select_by_id(conn, &id))
    }
}
