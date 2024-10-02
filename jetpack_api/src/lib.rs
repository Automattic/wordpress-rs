#![allow(dead_code, unused_variables)]

pub use jetpack_client::{JetpackClient, JetpackRequestBuilder};
use wp_api::{RequestExecutionError, WpApiError};

mod jetpack_client; // re-exported relevant types

pub mod jetpack_connection;
pub mod request;

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum JpApiErrorWrapper {
    #[error("{}", inner)]
    Inner { inner: WpApiError },
}

impl From<WpApiError> for JpApiErrorWrapper {
    fn from(value: WpApiError) -> Self {
        Self::Inner { inner: value }
    }
}

impl From<RequestExecutionError> for JpApiErrorWrapper {
    fn from(value: RequestExecutionError) -> Self {
        WpApiError::from(value).into()
    }
}

uniffi::setup_scaffolding!();
