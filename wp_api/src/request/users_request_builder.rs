use std::sync::Arc;

use crate::{
    SparseUser, SparseUserField, UserCreateParams, UserDeleteParams, UserDeleteResponse, UserId,
    UserListParams, UserUpdateParams, UserWithEditContext, UserWithEmbedContext,
    UserWithViewContext, WpApiError, WpContext,
};

use super::{
    endpoint::{users_endpoint::UsersEndpoint, ApiBaseUrl},
    RequestBuilder,
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
    pub async fn list_with_edit_context(
        &self,
        params: &UserListParams,
    ) -> Result<Vec<UserWithEditContext>, WpApiError> {
        self.request_builder
            .get(self.endpoint.list(WpContext::Edit, params))
            .await
    }

    pub async fn list_with_embed_context(
        &self,
        params: &UserListParams,
    ) -> Result<Vec<UserWithEmbedContext>, WpApiError> {
        self.request_builder
            .get(self.endpoint.list(WpContext::Embed, params))
            .await
    }

    pub async fn list_with_view_context(
        &self,
        params: &UserListParams,
    ) -> Result<Vec<UserWithViewContext>, WpApiError> {
        self.request_builder
            .get(self.endpoint.list(WpContext::View, params))
            .await
    }

    pub async fn filter_list(
        &self,
        context: WpContext,
        params: &UserListParams,
        fields: &[SparseUserField],
    ) -> Result<Vec<SparseUser>, WpApiError> {
        self.request_builder
            .get(self.endpoint.filter_list(context, &params, fields))
            .await
    }

    pub async fn retrieve_with_edit_context(
        &self,
        user_id: UserId,
    ) -> Result<UserWithEditContext, WpApiError> {
        self.request_builder
            .get(self.endpoint.retrieve(user_id, WpContext::Edit))
            .await
    }

    pub async fn retrieve_with_embed_context(
        &self,
        user_id: UserId,
    ) -> Result<UserWithEmbedContext, WpApiError> {
        self.request_builder
            .get(self.endpoint.retrieve(user_id, WpContext::Embed))
            .await
    }

    pub async fn retrieve_with_view_context(
        &self,
        user_id: UserId,
    ) -> Result<UserWithViewContext, WpApiError> {
        self.request_builder
            .get(self.endpoint.retrieve(user_id, WpContext::View))
            .await
    }

    pub async fn filter_retrieve(
        &self,
        user_id: UserId,
        context: WpContext,
        fields: &[SparseUserField],
    ) -> Result<SparseUser, WpApiError> {
        self.request_builder
            .get(self.endpoint.filter_retrieve(user_id, context, fields))
            .await
    }

    pub async fn retrieve_me_with_edit_context(&self) -> Result<UserWithEditContext, WpApiError> {
        self.request_builder
            .get(self.endpoint.retrieve_me(WpContext::Edit))
            .await
    }

    pub async fn retrieve_me_with_embed_context(&self) -> Result<UserWithEmbedContext, WpApiError> {
        self.request_builder
            .get(self.endpoint.retrieve_me(WpContext::Embed))
            .await
    }

    pub async fn retrieve_me_with_view_context(&self) -> Result<UserWithViewContext, WpApiError> {
        self.request_builder
            .get(self.endpoint.retrieve_me(WpContext::View))
            .await
    }

    pub async fn filter_retrieve_me(
        &self,
        context: WpContext,
        fields: &[SparseUserField],
    ) -> Result<SparseUser, WpApiError> {
        self.request_builder
            .get(self.endpoint.filter_retrieve_me(context, fields))
            .await
    }

    pub async fn create(
        &self,
        params: &UserCreateParams,
    ) -> Result<UserWithEditContext, WpApiError> {
        self.request_builder
            .post(self.endpoint.create(), params)
            .await
    }

    pub async fn update(
        &self,
        user_id: UserId,
        params: &UserUpdateParams,
    ) -> Result<UserWithEditContext, WpApiError> {
        self.request_builder
            .post(self.endpoint.update(user_id), params)
            .await
    }

    pub async fn update_me(
        &self,
        params: &UserUpdateParams,
    ) -> Result<UserWithEditContext, WpApiError> {
        self.request_builder
            .post(self.endpoint.update_me(), params)
            .await
    }

    pub async fn delete(
        &self,
        user_id: UserId,
        params: &UserDeleteParams,
    ) -> Result<UserDeleteResponse, WpApiError> {
        self.request_builder
            .delete(self.endpoint.delete(user_id, params))
            .await
    }

    pub async fn delete_me(
        &self,
        params: &UserDeleteParams,
    ) -> Result<UserDeleteResponse, WpApiError> {
        self.request_builder
            .delete(self.endpoint.delete_me(params))
            .await
    }
}
