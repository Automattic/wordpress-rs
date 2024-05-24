use std::collections::HashMap;

use crate::{
    SparseUserField, UserCreateParams, UserDeleteParams, UserId, UserListParams, UserUpdateParams,
    WPContext,
};

use super::{
    endpoint::{users_endpoint::UsersEndpoint, ApiBaseUrl},
    RequestMethod, WPNetworkRequest,
};

#[derive(Debug)]
pub(crate) struct UsersRequest {
    endpoint: UsersEndpoint,
    header_map: HashMap<String, String>,
    header_map_for_post_request: HashMap<String, String>,
}

impl UsersRequest {
    pub fn new(
        api_base_url: ApiBaseUrl,
        header_map: HashMap<String, String>,
        header_map_for_post_request: HashMap<String, String>,
    ) -> Self {
        Self {
            endpoint: UsersEndpoint::new(api_base_url),
            header_map,
            header_map_for_post_request,
        }
    }

    pub fn list(
        &self,
        context: WPContext,
        params: &Option<UserListParams>, // UniFFI doesn't support Option<&T>
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
        params: &Option<UserListParams>, // UniFFI doesn't support Option<&T>
        fields: &[SparseUserField],
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

    pub fn retrieve(&self, user_id: UserId, context: WPContext) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self.endpoint.retrieve(user_id, context).into(),
            header_map: self.header_map.clone(),
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
                .endpoint
                .filter_retrieve(user_id, context, fields)
                .into(),
            header_map: self.header_map.clone(),
            body: None,
        }
    }

    pub fn retrieve_me(&self, context: WPContext) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self.endpoint.retrieve_me(context).into(),
            header_map: self.header_map.clone(),
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
            url: self.endpoint.filter_retrieve_me(context, fields).into(),
            header_map: self.header_map.clone(),
            body: None,
        }
    }

    pub fn create(&self, params: &UserCreateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.endpoint.create().into(),
            header_map: self.header_map_for_post_request.clone(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn update(&self, user_id: UserId, params: &UserUpdateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.endpoint.update(user_id).into(),
            header_map: self.header_map_for_post_request.clone(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn update_me(&self, params: &UserUpdateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.endpoint.update_me().into(),
            header_map: self.header_map_for_post_request.clone(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn delete(&self, user_id: UserId, params: &UserDeleteParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::DELETE,
            url: self.endpoint.delete(user_id, params).into(),
            header_map: self.header_map.clone(),
            body: None,
        }
    }

    pub fn delete_me(&self, params: &UserDeleteParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::DELETE,
            url: self.endpoint.delete_me(params).into(),
            header_map: self.header_map.clone(),
            body: None,
        }
    }
}
