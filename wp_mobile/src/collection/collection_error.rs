use wp_localization::{MessageBundle, WpMessages, WpSupportsLocalization};
use wp_localization_macro::WpDeriveLocalizable;

/// Errors that can occur during collection operations
#[derive(Debug, thiserror::Error, uniffi::Error, WpDeriveLocalizable)]
pub enum CollectionError {
    DatabaseError { err_message: String },
}

impl From<wp_mobile_cache::SqliteDbError> for CollectionError {
    fn from(err: wp_mobile_cache::SqliteDbError) -> Self {
        CollectionError::DatabaseError {
            err_message: err.to_string(),
        }
    }
}

impl WpSupportsLocalization for CollectionError {
    fn message_bundle(&self) -> MessageBundle<'_> {
        match self {
            CollectionError::DatabaseError { .. } => WpMessages::database_generic_message(),
        }
    }
}
