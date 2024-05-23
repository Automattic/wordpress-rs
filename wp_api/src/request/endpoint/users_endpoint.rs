use url::Url;

use crate::{SparseUserField, UserDeleteParams, UserId, UserListParams, WPContext};

use super::{ApiBaseUrl, UrlExtension};

#[derive(Debug)]
pub struct UsersEndpoint {
    api_base_url: ApiBaseUrl,
}

impl UsersEndpoint {
    pub fn new(api_base_url: ApiBaseUrl) -> Self {
        Self { api_base_url }
    }

    pub fn create(&self) -> Url {
        self.api_base_url.by_appending("users")
    }

    pub fn delete(&self, user_id: UserId, params: &UserDeleteParams) -> Url {
        let mut url = self
            .api_base_url
            .by_extending(["users", &user_id.to_string()]);
        url.query_pairs_mut().extend_pairs(params.query_pairs());
        url
    }

    pub fn delete_me(&self, params: &UserDeleteParams) -> Url {
        let mut url = self.api_base_url.by_extending(["users", "me"]);
        url.query_pairs_mut().extend_pairs(params.query_pairs());
        url
    }

    pub fn list(&self, context: WPContext, params: Option<&UserListParams>) -> Url {
        let mut url = self.api_base_url.by_appending("users");
        url.query_pairs_mut()
            .append_pair("context", context.as_str());
        if let Some(params) = params {
            url.query_pairs_mut().extend_pairs(params.query_pairs());
        }
        url
    }

    pub fn filter_list(
        &self,
        context: WPContext,
        params: Option<&UserListParams>,
        fields: &[SparseUserField],
    ) -> Url {
        self.list(context, params).append_filter_fields(fields)
    }

    pub fn retrieve(&self, user_id: UserId, context: WPContext) -> Url {
        let mut url = self
            .api_base_url
            .by_extending(["users", &user_id.to_string()]);
        url.query_pairs_mut()
            .append_pair("context", context.as_str());
        url
    }

    pub fn filter_retrieve(
        &self,
        user_id: UserId,
        context: WPContext,
        fields: &[SparseUserField],
    ) -> Url {
        self.retrieve(user_id, context).append_filter_fields(fields)
    }

    pub fn retrieve_me(&self, context: WPContext) -> Url {
        let mut url = self.api_base_url.by_extending(["users", "me"]);
        url.query_pairs_mut()
            .append_pair("context", context.as_str());
        url
    }

    pub fn filter_retrieve_me(&self, context: WPContext, fields: &[SparseUserField]) -> Url {
        self.retrieve_me(context).append_filter_fields(fields)
    }

    pub fn update(&self, user_id: UserId) -> Url {
        self.api_base_url
            .by_extending(["users", &user_id.to_string()])
    }

    pub fn update_me(&self) -> Url {
        self.api_base_url.by_extending(["users", "me"])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::tests::{fixture_api_base_url, validate_endpoint},
        ApiEndpoint,
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
            users_endpoint.list(WPContext::Edit, None),
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
            has_published_posts: Some(true),
        };
        validate_endpoint(
            users_endpoint.list(WPContext::Edit, Some(&params)),
            "/users?context=edit&page=2&per_page=60&search=foo&slug=bar%2Cbaz&has_published_posts=true",
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
            has_published_posts: Some(true),
        };
        validate_endpoint(
            users_endpoint.filter_list(WPContext::Edit, Some(&params), &[SparseUserField::Name, SparseUserField::Email]),
            "/users?context=edit&page=2&per_page=60&search=foo&slug=bar%2Cbaz&has_published_posts=true&_fields=name%2Cemail",
        );
    }

    #[rstest]
    fn retrieve_user(users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.retrieve(UserId(98), WPContext::View),
            "/users/98?context=view",
        );
    }

    #[rstest]
    fn filter_retrieve_user(users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.filter_retrieve(
                UserId(98),
                WPContext::View,
                &[SparseUserField::Nickname, SparseUserField::Url],
            ),
            "/users/98?context=view&_fields=nickname%2Curl",
        );
    }

    #[rstest]
    fn retrieve_current_user(users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.retrieve_me(WPContext::Embed),
            "/users/me?context=embed",
        );
    }

    #[rstest]
    fn filter_retrieve_current_user(users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.filter_retrieve_me(
                WPContext::Embed,
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
    fn users_endpoint(fixture_api_base_url: ApiBaseUrl) -> UsersEndpoint {
        ApiEndpoint::new(fixture_api_base_url).users
    }
}
