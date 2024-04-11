use url::Url;

use crate::{UserDeleteParams, UserId, UserListParams, UserUpdateParams, WPContext};

const WP_JSON_PATH_SEGMENTS: [&str; 3] = ["wp-json", "wp", "v2"];

pub struct WpOrgApiBaseUrl {
    api_base_url: Url,
}

impl WpOrgApiBaseUrl {
    pub fn new(site_base_url: &str) -> Result<Self, url::ParseError> {
        Url::parse(site_base_url).map(|parsed_url| {
            let api_base_url = parsed_url
                .extend(WP_JSON_PATH_SEGMENTS)
                .expect("parsed_url is already parsed, so this can't result in an error");
            Self { api_base_url }
        })
    }

    pub fn users(&self) -> UsersEndpoint {
        UsersEndpoint { api_base_url: self }
    }

    fn append_to_api_base_url(&self, segment: &str) -> Url {
        self.api_base_url
            .clone()
            .append(segment)
            .expect("api_base_url is already parsed, so this can't result in an error")
    }

    fn extend_api_base_url<I>(&self, segments: I) -> Url
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.api_base_url
            .clone()
            .extend(segments)
            .expect("api_base_url is already parsed, so this can't result in an error")
    }
}

pub struct UsersEndpoint<'a> {
    api_base_url: &'a WpOrgApiBaseUrl,
}

impl UsersEndpoint<'_> {
    pub fn create(&self) -> Url {
        self.api_base_url.append_to_api_base_url("users")
    }

    pub fn delete(&self, user_id: UserId, params: &UserDeleteParams) -> Url {
        let mut url = self
            .api_base_url
            .extend_api_base_url(["users", &user_id.to_string()]);
        url.query_pairs_mut().extend_pairs(params.query_pairs());
        url
    }

    pub fn delete_me(&self, params: &UserDeleteParams) -> Url {
        let mut url = self.api_base_url.extend_api_base_url(["users", "me"]);
        url.query_pairs_mut().extend_pairs(params.query_pairs());
        url
    }

    pub fn list(&self, context: WPContext, params: Option<&UserListParams>) -> Url {
        let mut url = self.api_base_url.append_to_api_base_url("users");
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
            .extend_api_base_url(["users", &user_id.to_string()]);
        url.query_pairs_mut()
            .append_pair("context", context.as_str());
        url
    }

    pub fn retrieve_me(&self, context: WPContext) -> Url {
        let mut url = self.api_base_url.extend_api_base_url(["users", "me"]);
        url.query_pairs_mut()
            .append_pair("context", context.as_str());
        url
    }

    pub fn update(&self, user_id: UserId, params: &UserUpdateParams) -> Url {
        self.api_base_url
            .extend_api_base_url(["users", &user_id.to_string()])
    }

    pub fn update_me(&self, params: &UserUpdateParams) -> Url {
        self.api_base_url.extend_api_base_url(["users", "me"])
    }
}

trait UrlExtension {
    fn append(self, segment: &str) -> Result<Url, ()>;
    fn extend<I>(self, segments: I) -> Result<Url, ()>
    where
        I: IntoIterator,
        I::Item: AsRef<str>;
}

impl UrlExtension for Url {
    fn append(mut self, segment: &str) -> Result<Url, ()> {
        self.path_segments_mut()?.push(segment);
        Ok(self)
    }

    fn extend<I>(mut self, segments: I) -> Result<Url, ()>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.path_segments_mut()?.extend(segments);
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_site_base_url() {
        let base_url = "https://example.com/blog";
        let site_base_url = WpOrgApiBaseUrl::new(base_url).unwrap();
        assert_eq!(
            Url::parse(format!("{}/{}", base_url, WP_JSON_PATH_SEGMENTS.join("/")).as_str())
                .unwrap()
                .as_str(),
            site_base_url.api_base_url.as_str()
        );
    }
}
