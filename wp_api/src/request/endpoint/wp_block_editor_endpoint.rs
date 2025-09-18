use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::api_error::ParsedRequestError;
use crate::api_error::WpApiError;
use crate::request::WpNetworkHeaderMap;
use crate::{JsonValue, wp_block_editor::WpBlockEditorSettingsParams};
use std::sync::Arc;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum WpBlockEditorRequest {
    #[get(url = "/settings", params = &WpBlockEditorSettingsParams, output = JsonValue)]
    GetSettings,
}

impl DerivedRequest for WpBlockEditorRequest {
    fn namespace() -> impl AsNamespace {
        WpNamespace::WpBlockEditorV1
    }
}

#[derive(uniffi::Record)]
pub struct WpBlockEditorRequestGetRawSettingsResponse {
    pub data: Vec<u8>,
    pub header_map: Arc<WpNetworkHeaderMap>,
}

#[uniffi::export]
impl WpBlockEditorRequestExecutor {
    /// Fetch the raw settings bytes for the block editor. Useful for passing the data to the block editor directly without parsing it.
    pub async fn get_raw_settings(
        &self,
        params: &WpBlockEditorSettingsParams,
    ) -> Result<WpBlockEditorRequestGetRawSettingsResponse, crate::api_error::WpApiError> {
        use crate::api_error::MaybeWpError;
        use crate::middleware::PerformsRequests;
        use crate::request::NetworkRequestAccessor;

        let request = self.request_builder.get_settings(params);
        let request_url: String = request.url().into();
        let response = self.perform(std::sync::Arc::new(request)).await?;
        let response_status_code = response.status_code;

        if let Some(err) = WpApiError::try_parse(&response) {
            let unauthorized = err.is_unauthorized_error().unwrap_or_default()
                || (response_status_code == 401
                    && self
                        .fetch_authentication_state()
                        .await
                        .map(|auth_state| auth_state.is_unauthorized())
                        .unwrap_or_default());

            if unauthorized {
                self.delegate
                    .app_notifier
                    .requested_with_invalid_authentication(request_url)
                    .await;
            }

            return Err(err);
        }

        Result::Ok(WpBlockEditorRequestGetRawSettingsResponse {
            data: response.body,
            header_map: response.response_header_map,
        })
    }
}
