struct PluginsRequest {}

impl PluginsRequest {
    pub fn list(
        &self,
        context: WPContext,
        params: &Option<PluginListParams>, // UniFFI doesn't support Option<&T>
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self
                .api_endpoint
                .plugins
                .list(context, params.as_ref())
                .into(),
            header_map: self.header_map(),
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
                .api_endpoint
                .plugins
                .filter_list(context, params.as_ref(), fields)
                .into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn create(&self, params: &PluginCreateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.api_endpoint.plugins.create().into(),
            header_map: self.header_map_for_post_request(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn retrieve(&self, context: WPContext, plugin: &PluginSlug) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self.api_endpoint.plugins.retrieve(context, plugin).into(),
            header_map: self.header_map(),
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
                .api_endpoint
                .plugins
                .filter_retrieve(context, plugin, fields)
                .into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn update(&self, plugin: &PluginSlug, params: &PluginUpdateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.api_endpoint.plugins.update(plugin).into(),
            header_map: self.header_map_for_post_request(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn delete(&self, plugin: &PluginSlug) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::DELETE,
            url: self.api_endpoint.plugins.delete(plugin).into(),
            header_map: self.header_map(),
            body: None,
        }
    }
}
