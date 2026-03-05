use std::sync::RwLock;

use crate::{
    login::OAuth2Endpoints,
    parsed_url::ParsedUrl,
    wp_com::{
        oauth2::{
            AuthorizationCodeExtractionError, AuthorizationCodeExtractionResult,
            TokenRequestParameters, WpComOauthScope, parse_authorization_url,
            wp_com_oauth_scope_list_to_string,
        },
        sites::WpComSiteIdentifier,
    },
};
use url::Url;

/// A complete OAuth2 client configuration for any WordPress hosting provider that
/// uses OAuth2 for login.
///
/// Bundles the server endpoints with client credentials and requested permissions,
/// providing methods to build authorization URLs, parse callback responses, and
/// construct token exchange parameters. Use [`wordpress_com_oauth2_configuration`]
/// for a pre-configured WordPress.com instance.
#[derive(Debug, Clone, uniffi::Record)]
pub struct OAuth2Configuration {
    /// The authorization and token endpoints for this OAuth2 server.
    pub endpoints: OAuth2Endpoints,
    /// The application's client ID, as registered with the OAuth2 provider.
    pub client_id: u64,
    /// The application's client secret.
    pub client_secret: String,
    /// The redirect URI that the authorization server will send the user back to.
    pub redirect_uri: String,
    /// The set of permissions to request during authorization.
    pub scope: Vec<WpComOauthScope>,
}

/// Creates an [`OAuth2Configuration`] pre-configured for WordPress.com.
///
/// The returned configuration uses the standard WordPress.com OAuth2 endpoints
/// (`https://public-api.wordpress.com/oauth2/authorize` and `.../token`).
/// You only need to supply your application's client credentials and desired scope.
#[uniffi::export]
pub fn wordpress_com_oauth2_configuration(
    client_id: u64,
    client_secret: String,
    redirect_uri: String,
    scope: Vec<WpComOauthScope>,
) -> OAuth2Configuration {
    OAuth2Configuration {
        endpoints: OAuth2Endpoints {
            authorization_url: "https://public-api.wordpress.com/oauth2/authorize".to_string(),
            token_url: "https://public-api.wordpress.com/oauth2/token".to_string(),
        },
        client_id,
        client_secret,
        redirect_uri,
        scope,
    }
}

#[uniffi::export]
impl OAuth2Configuration {
    /// Builds the OAuth2 authorization URL that should be opened in the user's browser.
    ///
    /// The `state` parameter is included for CSRF protection and should be verified
    /// when the user is redirected back. Optionally pass a `blog` to request
    /// authorization scoped to a specific WordPress.com site.
    pub fn build_token_request_url(
        &self,
        state: &str,
        blog: Option<WpComSiteIdentifier>,
    ) -> ParsedUrl {
        let mut url = Url::parse(&self.endpoints.authorization_url)
            .expect("Failed to parse authorization URL");

        {
            url.query_pairs_mut()
                .append_pair("client_id", &self.client_id.to_string())
                .append_pair("redirect_uri", &self.redirect_uri)
                .append_pair("response_type", "code")
                .append_pair(
                    "scope",
                    &wp_com_oauth_scope_list_to_string(self.scope.clone()),
                )
                .append_pair("state", state);

            if let Some(blog) = &blog {
                url.query_pairs_mut().append_pair("blog", &blog.to_string());
            }
        }

        ParsedUrl::new(url)
    }

    /// Parses the authorization code (and optional state) from an OAuth2 callback URL.
    ///
    /// If `expected_state` is provided, the returned state is validated against it.
    /// A mismatch or missing state will return an error, which may indicate a CSRF attack.
    #[uniffi::method(default(expected_state = None))]
    pub fn parse_token_response(
        &self,
        url: String,
        expected_state: Option<String>,
    ) -> Result<AuthorizationCodeExtractionResult, AuthorizationCodeExtractionError> {
        let result = parse_authorization_url(url)?;

        if let Some(expected) = expected_state {
            match &result.state {
                Some(actual) if actual == &expected => {}
                Some(actual) => {
                    return Err(AuthorizationCodeExtractionError::StateMismatch {
                        expected,
                        actual: actual.clone(),
                    });
                }
                None => {
                    return Err(AuthorizationCodeExtractionError::StateMissing { expected });
                }
            }
        }

        Ok(result)
    }

    /// Builds the parameters needed to exchange an authorization code for an access token.
    ///
    /// The returned [`TokenRequestParameters`] should be POST-ed to the token endpoint.
    pub fn build_token_request_parameters(&self, code: String) -> TokenRequestParameters {
        TokenRequestParameters {
            client_id: self.client_id,
            client_secret: self.client_secret.clone(),
            code,
            grant_type: "authorization_code".to_string(),
            redirect_uri: self.redirect_uri.clone(),
        }
    }
}

/// A thread-safe store for registered [`OAuth2Configuration`] instances.
///
/// Use this to pre-register OAuth2 configurations (e.g., for WordPress.com) and
/// later look them up by their endpoints — for example, after autodiscovery returns
/// an [`OAuth2Endpoints`] value.
#[derive(uniffi::Object)]
pub struct OAuth2ConfigurationStore {
    configurations: RwLock<Vec<OAuth2Configuration>>,
}

impl Default for OAuth2ConfigurationStore {
    fn default() -> Self {
        Self::new()
    }
}

#[uniffi::export]
impl OAuth2ConfigurationStore {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            configurations: RwLock::new(Vec::new()),
        }
    }

    /// Registers a configuration in the store.
    pub fn add_configuration(&self, config: OAuth2Configuration) {
        self.configurations
            .write()
            .expect("RwLock poisoned")
            .push(config);
    }

    /// Finds a registered configuration whose endpoints match the given [`OAuth2Endpoints`].
    ///
    /// Both the `authorization_url` and `token_url` must match for a configuration to
    /// be returned.
    pub fn find_configuration(&self, endpoints: &OAuth2Endpoints) -> Option<OAuth2Configuration> {
        self.configurations
            .read()
            .expect("RwLock poisoned")
            .iter()
            .find(|c| {
                c.endpoints.authorization_url == endpoints.authorization_url
                    && c.endpoints.token_url == endpoints.token_url
            })
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_site_identifier_id_to_string() {
        let id = WpComSiteIdentifier::Id { value: 12345 };
        assert_eq!(id.to_string(), "12345");
    }

    #[test]
    fn test_site_identifier_slug_to_string() {
        let slug = WpComSiteIdentifier::Slug {
            value: "mysite.wordpress.com".to_string(),
        };
        assert_eq!(slug.to_string(), "mysite.wordpress.com");
    }

    fn test_endpoints() -> OAuth2Endpoints {
        OAuth2Endpoints {
            authorization_url: "https://public-api.wordpress.com/oauth2/authorize".to_string(),
            token_url: "https://public-api.wordpress.com/oauth2/token".to_string(),
        }
    }

    fn test_config() -> OAuth2Configuration {
        OAuth2Configuration {
            endpoints: test_endpoints(),
            client_id: 12345,
            client_secret: "my_secret".to_string(),
            redirect_uri: "https://myapp.com/callback".to_string(),
            scope: vec![WpComOauthScope::Global],
        }
    }

    #[test]
    fn test_build_token_request_url() {
        let config = test_config();
        let url = config.build_token_request_url("abc123", None);

        assert_eq!(
            url.url(),
            "https://public-api.wordpress.com/oauth2/authorize?client_id=12345&redirect_uri=https%3A%2F%2Fmyapp.com%2Fcallback&response_type=code&scope=global&state=abc123"
        );
    }

    #[test]
    fn test_build_token_request_url_with_blog_id() {
        let config = test_config();
        let url = config
            .build_token_request_url("abc123", Some(WpComSiteIdentifier::Id { value: 67890 }));

        assert_eq!(
            url.url(),
            "https://public-api.wordpress.com/oauth2/authorize?client_id=12345&redirect_uri=https%3A%2F%2Fmyapp.com%2Fcallback&response_type=code&scope=global&state=abc123&blog=67890"
        );
    }

    #[test]
    fn test_build_token_request_url_with_blog_slug() {
        let config = test_config();
        let url = config.build_token_request_url(
            "abc123",
            Some(WpComSiteIdentifier::Slug {
                value: "mysite.wordpress.com".to_string(),
            }),
        );

        assert_eq!(
            url.url(),
            "https://public-api.wordpress.com/oauth2/authorize?client_id=12345&redirect_uri=https%3A%2F%2Fmyapp.com%2Fcallback&response_type=code&scope=global&state=abc123&blog=mysite.wordpress.com"
        );
    }

    #[test]
    fn test_parse_token_response_extracts_code_and_state() {
        let config = test_config();
        let result = config
            .parse_token_response(
                "https://myapp.com/callback?code=auth_code_123&state=my_state".to_string(),
                None,
            )
            .expect("Should succeed");

        assert_eq!(result.code, "auth_code_123");
        assert_eq!(result.state, Some("my_state".to_string()));
    }

    #[test]
    fn test_parse_token_response_missing_code() {
        let config = test_config();
        let result = config.parse_token_response(
            "https://myapp.com/callback?state=my_state".to_string(),
            None,
        );

        assert!(matches!(
            result,
            Err(AuthorizationCodeExtractionError::MissingCode)
        ));
    }

    #[test]
    fn test_parse_token_response_state_matches() {
        let config = test_config();
        let result = config
            .parse_token_response(
                "https://myapp.com/callback?code=abc&state=expected_state".to_string(),
                Some("expected_state".to_string()),
            )
            .expect("Should succeed");

        assert_eq!(result.code, "abc");
        assert_eq!(result.state, Some("expected_state".to_string()));
    }

    #[test]
    fn test_parse_token_response_state_mismatch() {
        let config = test_config();
        let result = config.parse_token_response(
            "https://myapp.com/callback?code=abc&state=wrong_state".to_string(),
            Some("expected_state".to_string()),
        );

        assert!(matches!(
            result,
            Err(AuthorizationCodeExtractionError::StateMismatch { .. })
        ));
    }

    #[test]
    fn test_parse_token_response_state_missing_when_expected() {
        let config = test_config();
        let result = config.parse_token_response(
            "https://myapp.com/callback?code=abc".to_string(),
            Some("expected_state".to_string()),
        );

        assert!(matches!(
            result,
            Err(AuthorizationCodeExtractionError::StateMissing { .. })
        ));
    }

    #[test]
    fn test_parse_token_response_no_state_expected_none_returned() {
        let config = test_config();
        let result = config
            .parse_token_response("https://myapp.com/callback?code=abc".to_string(), None)
            .expect("Should succeed");

        assert_eq!(result.code, "abc");
        assert_eq!(result.state, None);
    }

    #[test]
    fn test_build_token_request_parameters() {
        let config = test_config();
        let params = config.build_token_request_parameters("auth_code_123".to_string());

        assert_eq!(
            params,
            TokenRequestParameters {
                client_id: 12345,
                client_secret: "my_secret".to_string(),
                code: "auth_code_123".to_string(),
                grant_type: "authorization_code".to_string(),
                redirect_uri: "https://myapp.com/callback".to_string(),
            }
        );
    }

    #[test]
    fn test_store_find_matching_configuration() {
        let store = OAuth2ConfigurationStore::new();
        store.add_configuration(test_config());

        let found = store.find_configuration(&test_endpoints());
        assert!(found.is_some());
        assert_eq!(found.unwrap().client_id, 12345);
    }

    #[test]
    fn test_store_returns_none_when_no_match() {
        let store = OAuth2ConfigurationStore::new();
        store.add_configuration(test_config());

        let other_endpoints = OAuth2Endpoints {
            authorization_url: "https://other.example.com/authorize".to_string(),
            token_url: "https://other.example.com/token".to_string(),
        };

        assert!(store.find_configuration(&other_endpoints).is_none());
    }

    #[test]
    fn test_store_multiple_configs_finds_correct_one() {
        let store = OAuth2ConfigurationStore::new();

        let endpoints_a = OAuth2Endpoints {
            authorization_url: "https://site-a.com/authorize".to_string(),
            token_url: "https://site-a.com/token".to_string(),
        };
        let config_a = OAuth2Configuration {
            endpoints: endpoints_a,
            client_id: 111,
            client_secret: "secret_a".to_string(),
            redirect_uri: "https://myapp.com/callback".to_string(),
            scope: vec![WpComOauthScope::Global],
        };

        let endpoints_b = OAuth2Endpoints {
            authorization_url: "https://site-b.com/authorize".to_string(),
            token_url: "https://site-b.com/token".to_string(),
        };
        let config_b = OAuth2Configuration {
            endpoints: endpoints_b.clone(),
            client_id: 222,
            client_secret: "secret_b".to_string(),
            redirect_uri: "https://myapp.com/callback".to_string(),
            scope: vec![WpComOauthScope::Global],
        };

        store.add_configuration(config_a);
        store.add_configuration(config_b);

        let found = store.find_configuration(&endpoints_b);
        assert!(found.is_some());
        assert_eq!(found.unwrap().client_id, 222);
    }

    #[test]
    fn test_wordpress_com_configuration() {
        let config = wordpress_com_oauth2_configuration(
            12345,
            "my_secret".to_string(),
            "https://myapp.com/callback".to_string(),
            vec![WpComOauthScope::Global],
        );

        assert_eq!(
            config.endpoints.authorization_url,
            "https://public-api.wordpress.com/oauth2/authorize"
        );
        assert_eq!(
            config.endpoints.token_url,
            "https://public-api.wordpress.com/oauth2/token"
        );
        assert_eq!(config.client_id, 12345);
        assert_eq!(config.client_secret, "my_secret");
        assert_eq!(config.redirect_uri, "https://myapp.com/callback");
        assert_eq!(config.scope, vec![WpComOauthScope::Global]);
    }

    #[test]
    fn test_store_requires_both_endpoints_to_match() {
        let store = OAuth2ConfigurationStore::new();
        store.add_configuration(test_config());

        // Same authorization_url but different token_url
        let partial_match = OAuth2Endpoints {
            authorization_url: "https://public-api.wordpress.com/oauth2/authorize".to_string(),
            token_url: "https://different.example.com/token".to_string(),
        };

        assert!(store.find_configuration(&partial_match).is_none());
    }
}
