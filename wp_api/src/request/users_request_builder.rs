use std::sync::Arc;

use crate::{
    RequestBuilder, SparseUserField, UserCreateParams, UserDeleteParams, UserId, UserListParams,
    UserUpdateParams, WpContext,
};

use super::{
    endpoint::{users_endpoint::UsersEndpoint, ApiBaseUrl},
    WpNetworkRequest,
};

#[derive(Debug, uniffi::Object)]
pub struct UsersRequestBuilder {
    endpoint: UsersEndpoint,
    request_builder: Arc<RequestBuilder>,
}

impl UsersRequestBuilder {
    pub(crate) fn new(api_base_url: Arc<ApiBaseUrl>, request_builder: Arc<RequestBuilder>) -> Self {
        Self {
            endpoint: UsersEndpoint::new(api_base_url),
            request_builder,
        }
    }
}

#[uniffi::export]
impl UsersRequestBuilder {
    pub fn list(
        &self,
        context: WpContext,
        params: &Option<UserListParams>, // UniFFI doesn't support Option<&T>
    ) -> WpNetworkRequest {
        self.request_builder
            .get(self.endpoint.list(context, params.as_ref()))
    }

    pub fn filter_list(
        &self,
        context: WpContext,
        params: &Option<UserListParams>, // UniFFI doesn't support Option<&T>
        fields: &[SparseUserField],
    ) -> WpNetworkRequest {
        self.request_builder
            .get(self.endpoint.filter_list(context, params.as_ref(), fields))
    }

    pub fn retrieve(&self, user_id: UserId, context: WpContext) -> WpNetworkRequest {
        self.request_builder
            .get(self.endpoint.retrieve(user_id, context))
    }

    pub fn filter_retrieve(
        &self,
        user_id: UserId,
        context: WpContext,
        fields: &[SparseUserField],
    ) -> WpNetworkRequest {
        self.request_builder
            .get(self.endpoint.filter_retrieve(user_id, context, fields))
    }

    pub fn retrieve_me(&self, context: WpContext) -> WpNetworkRequest {
        self.request_builder.get(self.endpoint.retrieve_me(context))
    }

    pub fn filter_retrieve_me(
        &self,
        context: WpContext,
        fields: &[SparseUserField],
    ) -> WpNetworkRequest {
        self.request_builder
            .get(self.endpoint.filter_retrieve_me(context, fields))
    }

    pub fn create(&self, params: &UserCreateParams) -> WpNetworkRequest {
        self.request_builder.post(self.endpoint.create(), params)
    }

    pub fn update(&self, user_id: UserId, params: &UserUpdateParams) -> WpNetworkRequest {
        self.request_builder
            .post(self.endpoint.update(user_id), params)
    }

    pub fn update_me(&self, params: &UserUpdateParams) -> WpNetworkRequest {
        self.request_builder.post(self.endpoint.update_me(), params)
    }

    pub fn delete(&self, user_id: UserId, params: &UserDeleteParams) -> WpNetworkRequest {
        self.request_builder
            .delete(self.endpoint.delete(user_id, params))
    }

    pub fn delete_me(&self, params: &UserDeleteParams) -> WpNetworkRequest {
        self.request_builder.delete(self.endpoint.delete_me(params))
    }
}
