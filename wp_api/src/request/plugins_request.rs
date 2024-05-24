use std::collections::HashMap;

use crate::{
    PluginCreateParams, PluginListParams, PluginSlug, PluginUpdateParams, SparsePluginField,
    WPContext,
};

use super::{endpoint::plugins_endpoint::PluginsEndpoint, RequestMethod, WPNetworkRequest};

#[derive(Debug)]
pub(crate) struct PluginsRequest {
    pub endpoint: PluginsEndpoint,
    pub header_map: HashMap<String, String>,
    pub header_map_for_post_request: HashMap<String, String>,
}

impl PluginsRequest {
    pub fn list(
        &self,
        context: WPContext,
        params: &Option<PluginListParams>, // UniFFI doesn't support Option<&T>
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self.endpoint.list(context, params.as_ref()).into(),
            header_map: self.header_map.clone(),
            body: None,
        }
    }

    pub fn filter_list(
        &self,
        context: WPContext,
        params: &Option<PluginListParams>, // UniFFI doesn't support Option<&T>
        fields: &[SparsePluginField],
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self
                .endpoint
                .filter_list(context, params.as_ref(), fields)
                .into(),
            header_map: self.header_map.clone(),
            body: None,
        }
    }

    pub fn create(&self, params: &PluginCreateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.endpoint.create().into(),
            header_map: self.header_map_for_post_request.clone(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn retrieve(&self, context: WPContext, plugin: &PluginSlug) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self.endpoint.retrieve(context, plugin).into(),
            header_map: self.header_map.clone(),
            body: None,
        }
    }

    pub fn filter_retrieve(
        &self,
        context: WPContext,
        plugin: &PluginSlug,
        fields: &[SparsePluginField],
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self
                .endpoint
                .filter_retrieve(context, plugin, fields)
                .into(),
            header_map: self.header_map.clone(),
            body: None,
        }
    }

    pub fn update(&self, plugin: &PluginSlug, params: &PluginUpdateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.endpoint.update(plugin).into(),
            header_map: self.header_map_for_post_request.clone(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn delete(&self, plugin: &PluginSlug) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::DELETE,
            url: self.endpoint.delete(plugin).into(),
            header_map: self.header_map.clone(),
            body: None,
        }
    }
}
