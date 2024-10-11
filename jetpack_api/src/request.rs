use std::{fmt::Debug, sync::Arc};

use crate::JetpackRequestExecutionError;
use serde::Deserialize;
use wp_api::{
    request::{WpNetworkRequest, WpNetworkResponse},
    ParsedRequestError,
};

pub mod endpoint;

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait JetpackRequestExecutor: Send + Sync + Debug {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<JetpackNetworkResponse, JetpackRequestExecutionError>;
}

#[derive(Debug, uniffi::Record)]
pub struct JetpackNetworkResponse {
    pub inner: WpNetworkResponse,
}

impl From<WpNetworkResponse> for JetpackNetworkResponse {
    fn from(value: WpNetworkResponse) -> Self {
        Self { inner: value }
    }
}

impl JetpackNetworkResponse {
    pub fn parse<'de, T, E>(&'de self) -> Result<T, E>
    where
        T: Deserialize<'de>,
        E: ParsedRequestError,
    {
        self.inner.parse()
    }
}
