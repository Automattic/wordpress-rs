struct UsersRequest {}

impl UsersRequest {
    pub fn list(
        &self,
        context: WPContext,
        params: &Option<UserListParams>, // UniFFI doesn't support Option<&T>
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self
                .api_endpoint
                .users
                .list(context, params.as_ref())
                .into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn filter_list(
        &self,
        context: WPContext,
        params: &Option<UserListParams>, // UniFFI doesn't support Option<&T>
        fields: &[SparseUserField],
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self
                .api_endpoint
                .users
                .filter_list(context, params.as_ref(), fields)
                .into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn retrieve(&self, user_id: UserId, context: WPContext) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self.api_endpoint.users.retrieve(user_id, context).into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn filter_retrieve(
        &self,
        user_id: UserId,
        context: WPContext,
        fields: &[SparseUserField],
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self
                .api_endpoint
                .users
                .filter_retrieve(user_id, context, fields)
                .into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn retrieve_me(&self, context: WPContext) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self.api_endpoint.users.retrieve_me(context).into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn filter_retrieve_me(
        &self,
        context: WPContext,
        fields: &[SparseUserField],
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self
                .api_endpoint
                .users
                .filter_retrieve_me(context, fields)
                .into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn create(&self, params: &UserCreateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.api_endpoint.users.create().into(),
            header_map: self.header_map_for_post_request(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn update(&self, user_id: UserId, params: &UserUpdateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.api_endpoint.users.update(user_id).into(),
            header_map: self.header_map_for_post_request(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn update_me(&self, params: &UserUpdateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.api_endpoint.users.update_me().into(),
            header_map: self.header_map_for_post_request(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn delete(&self, user_id: UserId, params: &UserDeleteParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::DELETE,
            url: self.api_endpoint.users.delete(user_id, params).into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn delete_me(&self, params: &UserDeleteParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::DELETE,
            url: self.api_endpoint.users.delete_me(params).into(),
            header_map: self.header_map(),
            body: None,
        }
    }
}
