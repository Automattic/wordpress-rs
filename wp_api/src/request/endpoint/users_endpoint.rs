use std::sync::Arc;

use crate::{SparseUserField, UserDeleteParams, UserId, UserListParams, WpContext};

use super::{ApiBaseUrl, ApiEndpointUrl, UrlExtension};

// Temporary mod to showcase `wp_derive_request_builder`
// The generated code can be inspected by running:
// ```
// cargo expand request::endpoint::users_endpoint::generated -p wp_api
// ```
mod generated {
    use super::*;

    #[derive(wp_derive_request_builder::WpDerivedRequest)]
    #[SparseField(SparseUserField)]
    enum UsersRequest {
        #[contextual_get(url = "/users", params = &UserListParams, output = Vec<crate::SparseUser>)]
        List,
        #[post(url = "/users", params = &crate::UserCreateParams, output = UserWithEditContext)]
        Create,
        #[delete(url = "/users/<user_id>", params = &UserDeleteParams, output = crate::UserDeleteResponse)]
        Delete,
        #[delete(url = "/users/me", params = &UserDeleteParams, output = crate::UserDeleteResponse)]
        DeleteMe,
        #[contextual_get(url = "/users/<user_id>", output = crate::SparseUser)]
        Retrieve,
        #[contextual_get(url = "/users/me", output = crate::SparseUser)]
        RetrieveMe,
        #[post(url = "/users/<user_id>", params = &crate::UserUpdateParams, output = UserWithEditContext)]
        Update,
        #[post(url = "/users/me", params = &crate::UserUpdateParams, output = UserWithEditContext)]
        UpdateMe,
    }
}

#[derive(Debug)]
pub(crate) struct UsersEndpoint {
    api_base_url: Arc<ApiBaseUrl>,
}

impl UsersEndpoint {
    pub fn new(api_base_url: Arc<ApiBaseUrl>) -> Self {
        Self { api_base_url }
    }

    pub fn create(&self) -> ApiEndpointUrl {
        self.api_base_url.by_appending("users").into()
    }

    pub fn delete(&self, user_id: UserId, params: &UserDeleteParams) -> ApiEndpointUrl {
        let mut url = self
            .api_base_url
            .by_extending(["users", &user_id.to_string()]);
        url.query_pairs_mut().extend_pairs(params.query_pairs());
        url.into()
    }

    pub fn delete_me(&self, params: &UserDeleteParams) -> ApiEndpointUrl {
        let mut url = self.api_base_url.by_extending(["users", "me"]);
        url.query_pairs_mut().extend_pairs(params.query_pairs());
        url.into()
    }

    pub fn list(&self, context: WpContext, params: &UserListParams) -> ApiEndpointUrl {
        let mut url = self.api_base_url.by_appending("users");
        url.query_pairs_mut()
            .append_pair("context", context.as_str());
        url.query_pairs_mut().extend_pairs(params.query_pairs());
        url.into()
    }

    pub fn filter_list(
        &self,
        context: WpContext,
        params: &UserListParams,
        fields: &[SparseUserField],
    ) -> ApiEndpointUrl {
        self.list(context, params)
            .url
            .append_filter_fields(fields)
            .into()
    }

    pub fn retrieve(&self, user_id: UserId, context: WpContext) -> ApiEndpointUrl {
        let mut url = self
            .api_base_url
            .by_extending(["users", &user_id.to_string()]);
        url.query_pairs_mut()
            .append_pair("context", context.as_str());
        url.into()
    }

    pub fn filter_retrieve(
        &self,
        user_id: UserId,
        context: WpContext,
        fields: &[SparseUserField],
    ) -> ApiEndpointUrl {
        self.retrieve(user_id, context)
            .url
            .append_filter_fields(fields)
            .into()
    }

    pub fn retrieve_me(&self, context: WpContext) -> ApiEndpointUrl {
        let mut url = self.api_base_url.by_extending(["users", "me"]);
        url.query_pairs_mut()
            .append_pair("context", context.as_str());
        url.into()
    }

    pub fn filter_retrieve_me(
        &self,
        context: WpContext,
        fields: &[SparseUserField],
    ) -> ApiEndpointUrl {
        self.retrieve_me(context)
            .url
            .append_filter_fields(fields)
            .into()
    }

    pub fn update(&self, user_id: UserId) -> ApiEndpointUrl {
        self.api_base_url
            .by_extending(["users", &user_id.to_string()])
            .into()
    }

    pub fn update_me(&self) -> ApiEndpointUrl {
        self.api_base_url.by_extending(["users", "me"]).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::tests::{fixture_api_base_url, validate_endpoint},
        WpApiParamUsersHasPublishedPosts,
    };
    use rstest::*;

    #[rstest]
    fn create_user(users_endpoint: UsersEndpoint) {
        validate_endpoint(users_endpoint.create(), "/users");
    }

    #[rstest]
    fn delete_user(users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.delete(
                UserId(54),
                &UserDeleteParams {
                    reassign: UserId(98),
                },
            ),
            "/users/54?reassign=98&force=true",
        );
    }

    #[rstest]
    fn delete_current_user(users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.delete_me(&UserDeleteParams {
                reassign: UserId(98),
            }),
            "/users/me?reassign=98&force=true",
        );
    }

    #[rstest]
    fn list_users(users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.list(WpContext::Edit, &UserListParams::default()),
            "/users?context=edit",
        );
    }

    #[rstest]
    fn list_users_default_params_empty_fields(users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.list(WpContext::Edit, &UserListParams::default()),
            "/users?context=edit",
        );
    }

    #[rstest]
    fn list_users_with_params(users_endpoint: UsersEndpoint) {
        let params = UserListParams {
            page: Some(2),
            per_page: Some(60),
            search: Some("foo".to_string()),
            exclude: Vec::new(),
            include: Vec::new(),
            offset: None,
            order: None,
            orderby: None,
            slug: vec!["bar".to_string(), "baz".to_string()],
            roles: Vec::new(),
            capabilities: Vec::new(),
            who: None,
            has_published_posts: Some(WpApiParamUsersHasPublishedPosts::True),
        };
        validate_endpoint(
            users_endpoint.list(WpContext::Edit, &params),
            "/users?context=edit&page=2&per_page=60&search=foo&slug=bar%2Cbaz&has_published_posts=true",
        );
    }

    #[rstest]
    fn filter_list_users_default_params_empty_fields(users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.filter_list(WpContext::Edit, &UserListParams::default(), &[]),
            "/users?context=edit&_fields=",
        );
    }

    #[rstest]
    fn filter_list_users_with_params(users_endpoint: UsersEndpoint) {
        let params = UserListParams {
            page: Some(2),
            per_page: Some(60),
            search: Some("foo".to_string()),
            exclude: Vec::new(),
            include: Vec::new(),
            offset: None,
            order: None,
            orderby: None,
            slug: vec!["bar".to_string(), "baz".to_string()],
            roles: Vec::new(),
            capabilities: Vec::new(),
            who: None,
            has_published_posts: Some(WpApiParamUsersHasPublishedPosts::PostTypes(vec![
                "post".to_string(),
                "page".to_string(),
            ])),
        };
        validate_endpoint(
            users_endpoint.filter_list(WpContext::Edit, &params, &[SparseUserField::Name, SparseUserField::Email]),
            "/users?context=edit&page=2&per_page=60&search=foo&slug=bar%2Cbaz&has_published_posts=post%2Cpage&_fields=name%2Cemail",
        );
    }

    #[rstest]
    fn retrieve_user(users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.retrieve(UserId(98), WpContext::View),
            "/users/98?context=view",
        );
    }

    #[rstest]
    fn filter_retrieve_user(users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.filter_retrieve(
                UserId(98),
                WpContext::View,
                &[SparseUserField::Nickname, SparseUserField::Url],
            ),
            "/users/98?context=view&_fields=nickname%2Curl",
        );
    }

    #[rstest]
    fn retrieve_current_user(users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.retrieve_me(WpContext::Embed),
            "/users/me?context=embed",
        );
    }

    #[rstest]
    fn filter_retrieve_current_user(users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.filter_retrieve_me(
                WpContext::Embed,
                &[SparseUserField::Roles, SparseUserField::Capabilities],
            ),
            "/users/me?context=embed&_fields=roles%2Ccapabilities",
        );
    }

    #[rstest]
    fn update_user(users_endpoint: UsersEndpoint) {
        validate_endpoint(users_endpoint.update(UserId(98)), "/users/98");
    }

    #[rstest]
    fn update_current_user(users_endpoint: UsersEndpoint) {
        validate_endpoint(users_endpoint.update_me(), "/users/me");
    }

    #[fixture]
    fn users_endpoint(fixture_api_base_url: Arc<ApiBaseUrl>) -> UsersEndpoint {
        UsersEndpoint::new(fixture_api_base_url)
    }
}
