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
