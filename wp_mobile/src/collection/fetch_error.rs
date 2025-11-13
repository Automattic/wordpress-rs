use wp_api::prelude::WpApiError;
use wp_mobile_cache::SqliteDbError;

/// Errors that can occur during network fetch operations
///
/// This combines API/network errors from wp_api with database errors
/// from cache operations.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FetchError {
    /// API or network error from wp_api
    #[error(transparent)]
    Api(WpApiError),

    /// Database error during cache upsert
    #[error("Database error: {err_message}")]
    Database { err_message: String },
}

impl From<WpApiError> for FetchError {
    fn from(err: WpApiError) -> Self {
        FetchError::Api(err)
    }
}

impl From<SqliteDbError> for FetchError {
    fn from(err: SqliteDbError) -> Self {
        FetchError::Database {
            err_message: err.to_string(),
        }
    }
}
