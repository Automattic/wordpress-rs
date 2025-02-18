#[derive(Debug, uniffi::Error, thiserror::Error)]
pub(crate) enum UniffiSerializationError {
    #[error("{reason:?}")]
    Serde { reason: String },
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
