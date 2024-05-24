use std::sync::Arc;

use crate::{
    PluginCreateParams, PluginListParams, PluginSlug, PluginUpdateParams, RequestBuilder,
    SparsePluginField, WPContext,
};

use super::{
    endpoint::{plugins_endpoint::PluginsEndpoint, ApiBaseUrl},
    WPNetworkRequest,
};

#[derive(Debug)]
pub(crate) struct PluginsRequest {
    endpoint: PluginsEndpoint,
    request_builder: Arc<RequestBuilder>,
}

impl PluginsRequest {
    pub fn new(api_base_url: ApiBaseUrl, request_builder: Arc<RequestBuilder>) -> Self {
        Self {
            endpoint: PluginsEndpoint::new(api_base_url),
            request_builder,
        }
    }

    pub fn list(
        &self,
        context: WPContext,
        params: &Option<PluginListParams>, // UniFFI doesn't support Option<&T>
    ) -> WPNetworkRequest {
        self.request_builder
            .get(self.endpoint.list(context, params.as_ref()))
    }

    pub fn filter_list(
        &self,
        context: WPContext,
        params: &Option<PluginListParams>, // UniFFI doesn't support Option<&T>
        fields: &[SparsePluginField],
    ) -> WPNetworkRequest {
        self.request_builder
            .get(self.endpoint.filter_list(context, params.as_ref(), fields))
    }

    pub fn create(&self, params: &PluginCreateParams) -> WPNetworkRequest {
        self.request_builder.post(self.endpoint.create(), params)
    }

    pub fn retrieve(&self, context: WPContext, plugin: &PluginSlug) -> WPNetworkRequest {
        self.request_builder
            .get(self.endpoint.retrieve(context, plugin))
    }

    pub fn filter_retrieve(
        &self,
        context: WPContext,
        plugin: &PluginSlug,
        fields: &[SparsePluginField],
    ) -> WPNetworkRequest {
        self.request_builder
            .get(self.endpoint.filter_retrieve(context, plugin, fields))
    }

    pub fn update(&self, plugin: &PluginSlug, params: &PluginUpdateParams) -> WPNetworkRequest {
        self.request_builder
            .post(self.endpoint.update(plugin), params)
    }

    pub fn delete(&self, plugin: &PluginSlug) -> WPNetworkRequest {
        self.request_builder.delete(self.endpoint.delete(plugin))
    }
}
