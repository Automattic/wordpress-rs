use url::Url;

use crate::{ApiBaseUrl, UserDeleteParams, UserId, UserListParams, WPContext};

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

    pub fn retrieve(&self, user_id: UserId, context: WPContext) -> Url {
        let mut url = self
            .api_base_url
            .by_extending(["users", &user_id.to_string()]);
        url.query_pairs_mut()
            .append_pair("context", context.as_str());
        url
    }

    pub fn retrieve_me(&self, context: WPContext) -> Url {
        let mut url = self.api_base_url.by_extending(["users", "me"]);
        url.query_pairs_mut()
            .append_pair("context", context.as_str());
        url
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
    use crate::ApiEndpoint;
    use rstest::*;

    #[rstest]
    fn create_user(api_base_url: ApiBaseUrl, users_endpoint: UsersEndpoint) {
        validate_endpoint(users_endpoint.create(), "/users", &api_base_url);
    }

    #[rstest]
    fn delete_user(api_base_url: ApiBaseUrl, users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.delete(
                UserId(54),
                &UserDeleteParams {
                    reassign: UserId(98),
                },
            ),
            "/users/54?reassign=98&force=true",
            &api_base_url,
        );
    }

    #[rstest]
    fn delete_current_user(api_base_url: ApiBaseUrl, users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.delete_me(&UserDeleteParams {
                reassign: UserId(98),
            }),
            "/users/me?reassign=98&force=true",
            &api_base_url,
        );
    }

    #[rstest]
    fn list_users(api_base_url: ApiBaseUrl, users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.list(WPContext::Edit, None),
            "/users?context=edit",
            &api_base_url,
        );
    }

    #[rstest]
    fn list_users_with_params(api_base_url: ApiBaseUrl, users_endpoint: UsersEndpoint) {
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
            "/users?context=edit&page=2&per_page=60&search=foo&slug=bar%2Cbaz&has_published_post=true",
            &api_base_url,
        );
    }

    #[rstest]
    fn retrieve_user(api_base_url: ApiBaseUrl, users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.retrieve(UserId(98), WPContext::View),
            "/users/98?context=view",
            &api_base_url,
        );
    }

    #[rstest]
    fn retrieve_current_user(api_base_url: ApiBaseUrl, users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.retrieve_me(WPContext::Embed),
            "/users/me?context=embed",
            &api_base_url,
        );
    }

    #[rstest]
    fn update_user(api_base_url: ApiBaseUrl, users_endpoint: UsersEndpoint) {
        validate_endpoint(
            users_endpoint.update(UserId(98)),
            "/users/98",
            &api_base_url,
        );
    }

    #[rstest]
    fn update_current_user(api_base_url: ApiBaseUrl, users_endpoint: UsersEndpoint) {
        validate_endpoint(users_endpoint.update_me(), "/users/me", &api_base_url);
    }

    #[fixture]
    fn api_base_url() -> ApiBaseUrl {
        ApiBaseUrl::new("https://foo.com").unwrap()
    }

    #[fixture]
    fn users_endpoint(api_base_url: ApiBaseUrl) -> UsersEndpoint {
        ApiEndpoint::new(api_base_url).users
    }

    fn validate_endpoint(endpoint_url: Url, path: &str, api_base_url: &ApiBaseUrl) {
        assert_eq!(
            endpoint_url.as_str(),
            format!("{}{}", api_base_url.as_str(), path)
        );
    }
}
