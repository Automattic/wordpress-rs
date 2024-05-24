use std::sync::Arc;

use crate::{
    RequestBuilder, SparseUserField, UserCreateParams, UserDeleteParams, UserId, UserListParams,
    UserUpdateParams, WPContext,
};

use super::{
    endpoint::{users_endpoint::UsersEndpoint, ApiBaseUrl},
    WPNetworkRequest,
};

#[derive(Debug)]
pub(crate) struct UsersRequest {
    endpoint: UsersEndpoint,
    request_builder: Arc<RequestBuilder>,
}

impl UsersRequest {
    pub fn new(api_base_url: ApiBaseUrl, request_builder: Arc<RequestBuilder>) -> Self {
        Self {
            endpoint: UsersEndpoint::new(api_base_url),
            request_builder,
        }
    }

    pub fn list(
        &self,
        context: WPContext,
        params: &Option<UserListParams>, // UniFFI doesn't support Option<&T>
    ) -> WPNetworkRequest {
        self.request_builder
            .get(self.endpoint.list(context, params.as_ref()))
    }

    pub fn filter_list(
        &self,
        context: WPContext,
        params: &Option<UserListParams>, // UniFFI doesn't support Option<&T>
        fields: &[SparseUserField],
    ) -> WPNetworkRequest {
        self.request_builder
            .get(self.endpoint.filter_list(context, params.as_ref(), fields))
    }

    pub fn retrieve(&self, user_id: UserId, context: WPContext) -> WPNetworkRequest {
        self.request_builder
            .get(self.endpoint.retrieve(user_id, context))
    }

    pub fn filter_retrieve(
        &self,
        user_id: UserId,
        context: WPContext,
        fields: &[SparseUserField],
    ) -> WPNetworkRequest {
        self.request_builder
            .get(self.endpoint.filter_retrieve(user_id, context, fields))
    }

    pub fn retrieve_me(&self, context: WPContext) -> WPNetworkRequest {
        self.request_builder.get(self.endpoint.retrieve_me(context))
    }

    pub fn filter_retrieve_me(
        &self,
        context: WPContext,
        fields: &[SparseUserField],
    ) -> WPNetworkRequest {
        self.request_builder
            .get(self.endpoint.filter_retrieve_me(context, fields))
    }

    pub fn create(&self, params: &UserCreateParams) -> WPNetworkRequest {
        self.request_builder.post(self.endpoint.create(), params)
    }

    pub fn update(&self, user_id: UserId, params: &UserUpdateParams) -> WPNetworkRequest {
        self.request_builder
            .post(self.endpoint.update(user_id), params)
    }

    pub fn update_me(&self, params: &UserUpdateParams) -> WPNetworkRequest {
        self.request_builder.post(self.endpoint.update_me(), params)
    }

    pub fn delete(&self, user_id: UserId, params: &UserDeleteParams) -> WPNetworkRequest {
        self.request_builder
            .delete(self.endpoint.delete(user_id, params))
    }

    pub fn delete_me(&self, params: &UserDeleteParams) -> WPNetworkRequest {
        self.request_builder.delete(self.endpoint.delete_me(params))
    }
}
