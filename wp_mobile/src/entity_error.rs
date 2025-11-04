/// Errors that can occur during entity operations
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum EntityError {
    #[error("Database error: {message}")]
    DatabaseError { message: String },
}

impl From<wp_mobile_cache::SqliteDbError> for EntityError {
    fn from(err: wp_mobile_cache::SqliteDbError) -> Self {
        EntityError::DatabaseError {
            message: err.to_string(),
        }
    }
}
