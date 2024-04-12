use url::Url;

use crate::{UserDeleteParams, UserId, UserListParams, UserUpdateParams, WPContext};

const WP_JSON_PATH_SEGMENTS: [&str; 3] = ["wp-json", "wp", "v2"];

#[derive(Debug, Clone)]
pub struct ApiBaseUrl {
    url: Url,
}

impl ApiBaseUrl {
    pub fn new(site_base_url: &str) -> Result<Self, url::ParseError> {
        Url::parse(site_base_url).map(|parsed_url| {
            let url = parsed_url
                .extend(WP_JSON_PATH_SEGMENTS)
                .expect("parsed_url is already parsed, so this can't result in an error");
            Self { url }
        })
    }

    fn by_appending(&self, segment: &str) -> Url {
        self.url
            .clone()
            .append(segment)
            .expect("api_base_url is already parsed, so this can't result in an error")
    }

    fn by_extending<I>(&self, segments: I) -> Url
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.url
            .clone()
            .extend(segments)
            .expect("api_base_url is already parsed, so this can't result in an error")
    }
}

pub struct ApiEndpoint {
    pub base_url: ApiBaseUrl,
    pub users: UsersEndpoint,
}

impl ApiEndpoint {
    pub fn new(site_base_url: &str) -> Result<Self, url::ParseError> {
        ApiBaseUrl::new(site_base_url).map(|api_base_url| Self {
            base_url: api_base_url.clone(),
            users: UsersEndpoint::new(api_base_url.clone()),
        })
    }
}

pub struct UsersEndpoint {
    api_base_url: ApiBaseUrl,
}

impl UsersEndpoint {
    fn new(api_base_url: ApiBaseUrl) -> Self {
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

    pub fn update(&self, user_id: UserId, params: &UserUpdateParams) -> Url {
        self.api_base_url
            .by_extending(["users", &user_id.to_string()])
    }

    pub fn update_me(&self, params: &UserUpdateParams) -> Url {
        self.api_base_url.by_extending(["users", "me"])
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
    fn append_url() {
        let url = Url::parse("https://foo.com").unwrap();
        assert_eq!(url.append("bar").unwrap().as_str(), "https://foo.com/bar");
    }

    #[test]
    fn extend_url() {
        let url = Url::parse("https://foo.com").unwrap();
        assert_eq!(
            url.extend(["bar", "baz"]).unwrap().as_str(),
            "https://foo.com/bar/baz"
        );
    }
}