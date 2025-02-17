#[derive(Debug, uniffi::Object, thiserror::Error)]
#[error("{inner:?}")]
pub(crate) struct UniffiSerializationError {
    inner: serde_json::Error,
}

impl From<serde_json::Error> for UniffiSerializationError {
    fn from(inner: serde_json::Error) -> Self {
        Self { inner }
    }
}

#[macro_export]
macro_rules! uniffi_export_serialization {
    ($func_name:ident, $type:ty) => {
        paste::paste! {
            #[uniffi::export]
            fn [<serialize_ $func_name>](value: $type) -> Result<Vec<u8>, Arc<$crate::uniffi_serde::UniffiSerializationError>> {
                serde_json::to_vec(&value)
                    .map_err(|e| Arc::new(e.into()))
            }

            #[uniffi::export]
            fn [<deserialize_ $func_name>](value: Vec<u8>) -> Result<$type, Arc<$crate::uniffi_serde::UniffiSerializationError>> {
                serde_json::from_slice(&value)
                    .map_err(|e| Arc::new(e.into()))
            }
        }
    };
}
