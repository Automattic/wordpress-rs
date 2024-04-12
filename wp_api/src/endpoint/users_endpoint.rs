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
    use crate::ApiEndpoint;

    use super::*;
    use rstest::*;

    #[rstest]
    fn create_user_endpoint(api_base_url: ApiBaseUrl, users_endpoint: UsersEndpoint) {
        assert_eq!(
            users_endpoint.create().as_str(),
            wp_json_endpoint_by_appending(api_base_url, "/users")
        );
    }

    #[rstest]
    fn update_user_me_endpoint(api_base_url: ApiBaseUrl, users_endpoint: UsersEndpoint) {
        assert_eq!(
            users_endpoint.update_me().as_str(),
            wp_json_endpoint_by_appending(api_base_url, "/users/me")
        );
    }

    #[fixture]
    fn api_base_url() -> ApiBaseUrl {
        ApiBaseUrl::new("https://foo.com").unwrap()
    }

    #[fixture]
    fn users_endpoint(api_base_url: ApiBaseUrl) -> UsersEndpoint {
        ApiEndpoint::new(api_base_url).users
    }

    fn wp_json_endpoint_by_appending(api_base_url: ApiBaseUrl, suffix: &str) -> String {
        format!("{}{}", api_base_url.as_str(), suffix)
    }
}
