/// Errors that can occur during entity operations
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum EntityError {
    #[error("Database error: {err_message}")]
    DatabaseError { err_message: String },
}

impl From<wp_mobile_cache::SqliteDbError> for EntityError {
    fn from(err: wp_mobile_cache::SqliteDbError) -> Self {
        EntityError::DatabaseError {
            err_message: err.to_string(),
        }
    }
}
