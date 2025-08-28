use wp_localization::{MessageBundle, WpMessages, WpSupportsLocalization};
use wp_localization_macro::WpDeriveLocalizable;

#[derive(Debug, thiserror::Error, uniffi::Error, WpDeriveLocalizable)]
pub enum UniffiSerializationError {
    Serde { reason: String },
}

impl WpSupportsLocalization for UniffiSerializationError {
    fn message_bundle(&self) -> MessageBundle<'_> {
        match self {
            UniffiSerializationError::Serde { reason } => {
                WpMessages::uniffi_serialization_error_serde(reason)
            }
        }
    }
}

impl From<serde_json::Error> for UniffiSerializationError {
    fn from(error: serde_json::Error) -> Self {
        UniffiSerializationError::Serde {
            reason: error.to_string(),
        }
    }
}

#[macro_export]
macro_rules! uniffi_export_serialization {
    ($func_name:ident, $type:ty) => {
        paste::paste! {
            #[uniffi::export]
            fn [<serialize_ $func_name>](value: $type) -> Result<Vec<u8>, $crate::uniffi_serde::UniffiSerializationError> {
                Ok(serde_json::to_vec(&value)?)
            }

            #[uniffi::export]
            fn [<deserialize_ $func_name>](value: Vec<u8>) -> Result<$type, $crate::uniffi_serde::UniffiSerializationError> {
                Ok(serde_json::from_slice(&value)?)
            }
        }
    };
}
