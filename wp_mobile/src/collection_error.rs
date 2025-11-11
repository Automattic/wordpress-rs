/// Errors that can occur during collection operations
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum CollectionError {
    #[error("Database error: {err_message}")]
    DatabaseError { err_message: String },
}

impl From<wp_mobile_cache::SqliteDbError> for CollectionError {
    fn from(err: wp_mobile_cache::SqliteDbError) -> Self {
        CollectionError::DatabaseError {
            err_message: err.to_string(),
        }
    }
}
