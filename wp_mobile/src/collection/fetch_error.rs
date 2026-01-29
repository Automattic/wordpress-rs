use wp_api::prelude::WpApiError;
use wp_localization::{MessageBundle, WpMessages, WpSupportsLocalization};
use wp_localization_macro::WpDeriveLocalizable;
use wp_mobile_cache::SqliteDbError;

/// Errors that can occur during network fetch operations
///
/// This combines API/network errors from wp_api with database errors
/// from cache operations.
#[derive(Debug, thiserror::Error, uniffi::Error, WpDeriveLocalizable)]
pub enum FetchError {
    /// API or network error from wp_api
    Api(WpApiError),

    /// Database error during cache upsert
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

impl From<crate::service::WpServiceError> for FetchError {
    fn from(err: crate::service::WpServiceError) -> Self {
        FetchError::Database {
            err_message: err.to_string(),
        }
    }
}

impl WpSupportsLocalization for FetchError {
    fn message_bundle(&self) -> MessageBundle<'_> {
        match self {
            FetchError::Api(api_err) => api_err.message_bundle(),
            FetchError::Database { err_message } => {
                WpMessages::database_generic_message(err_message)
            }
        }
    }
}
