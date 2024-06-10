use std::sync::Arc;

use crate::{
    PluginCreateParams, PluginDeleteResponse, PluginListParams, PluginSlug, PluginUpdateParams,
    PluginWithEditContext, PluginWithEmbedContext, PluginWithViewContext, SparsePlugin,
    SparsePluginField, WpApiError, WpContext,
};

use super::{
    endpoint::{plugins_endpoint::PluginsEndpoint, ApiBaseUrl},
    RequestBuilder,
};

#[derive(Debug, uniffi::Object)]
pub struct PluginsRequestBuilder {
    endpoint: PluginsEndpoint,
    request_builder: Arc<RequestBuilder>,
}

impl PluginsRequestBuilder {
    pub(crate) fn new(api_base_url: Arc<ApiBaseUrl>, request_builder: Arc<RequestBuilder>) -> Self {
        Self {
            endpoint: PluginsEndpoint::new(api_base_url),
            request_builder,
        }
    }
}

#[uniffi::export]
impl PluginsRequestBuilder {
    pub async fn list_with_edit_context(
        &self,
        params: &Option<PluginListParams>, // UniFFI doesn't support Option<&T>
    ) -> Result<Vec<PluginWithEditContext>, WpApiError> {
        self.request_builder
            .get(self.endpoint.list(WpContext::Edit, params.as_ref()))
            .await
    }

    pub async fn list_with_embed_context(
        &self,
        params: &Option<PluginListParams>, // UniFFI doesn't support Option<&T>
    ) -> Result<Vec<PluginWithEmbedContext>, WpApiError> {
        self.request_builder
            .get(self.endpoint.list(WpContext::Embed, params.as_ref()))
            .await
    }

    pub async fn list_with_view_context(
        &self,
        params: &Option<PluginListParams>, // UniFFI doesn't support Option<&T>
    ) -> Result<Vec<PluginWithViewContext>, WpApiError> {
        self.request_builder
            .get(self.endpoint.list(WpContext::View, params.as_ref()))
            .await
    }

    pub async fn filter_list(
        &self,
        context: WpContext,
        params: &Option<PluginListParams>, // UniFFI doesn't support Option<&T>
        fields: &[SparsePluginField],
    ) -> Result<Vec<SparsePlugin>, WpApiError> {
        self.request_builder
            .get(self.endpoint.filter_list(context, params.as_ref(), fields))
            .await
    }

    pub async fn create(
        &self,
        params: &PluginCreateParams,
    ) -> Result<PluginWithEditContext, WpApiError> {
        self.request_builder
            .post(self.endpoint.create(), params)
            .await
    }

    pub async fn retrieve_with_edit_context(
        &self,
        plugin: &PluginSlug,
    ) -> Result<PluginWithEditContext, WpApiError> {
        self.request_builder
            .get(self.endpoint.retrieve(WpContext::Edit, plugin))
            .await
    }

    pub async fn retrieve_with_embed_context(
        &self,
        plugin: &PluginSlug,
    ) -> Result<PluginWithEmbedContext, WpApiError> {
        self.request_builder
            .get(self.endpoint.retrieve(WpContext::Embed, plugin))
            .await
    }

    pub async fn retrieve_with_view_context(
        &self,
        plugin: &PluginSlug,
    ) -> Result<PluginWithViewContext, WpApiError> {
        self.request_builder
            .get(self.endpoint.retrieve(WpContext::View, plugin))
            .await
    }

    pub async fn filter_retrieve(
        &self,
        context: WpContext,
        plugin: &PluginSlug,
        fields: &[SparsePluginField],
    ) -> Result<SparsePlugin, WpApiError> {
        self.request_builder
            .get(self.endpoint.filter_retrieve(context, plugin, fields))
            .await
    }

    pub async fn update(
        &self,
        plugin: &PluginSlug,
        params: &PluginUpdateParams,
    ) -> Result<PluginWithEditContext, WpApiError> {
        self.request_builder
            .post(self.endpoint.update(plugin), params)
            .await
    }

    pub async fn delete(&self, plugin: &PluginSlug) -> Result<PluginDeleteResponse, WpApiError> {
        self.request_builder
            .delete(self.endpoint.delete(plugin))
            .await
    }
}
