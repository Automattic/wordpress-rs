use crate::{
    prelude::ParsedUrl,
    url_query::{AppendUrlQueryPairs, QueryPairs},
};
use serde::{Deserialize, Serialize};
use url::Url;
use wp_serde_helper::deserialize_u64_or_none_with_zero_as_none_from_string;
use wp_serde_helper::deserialize_u64_or_string;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum AuthorizationCodeExtractionError {
    #[error("Invalid URL: {reason}")]
    InvalidUrl { reason: String },
    #[error("Missing 'code' parameter in authorization URL")]
    MissingCode,
    #[error("{reason}")]
    Error { reason: String, code: String },
}

#[derive(Debug, Serialize, uniffi::Record)]
pub struct AuthorizationCodeExtractionResult {
    pub code: String,
    pub state: Option<String>,
}

impl From<url::ParseError> for AuthorizationCodeExtractionError {
    fn from(err: url::ParseError) -> Self {
        AuthorizationCodeExtractionError::InvalidUrl {
            reason: err.to_string(),
        }
    }
}

#[derive(Debug, Serialize, uniffi::Record)]
pub struct TokenValidationParameters {
    pub client_id: u64,
    pub token: String,
}

impl AppendUrlQueryPairs for TokenValidationParameters {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut.append_pair("client_id", &self.client_id.to_string());
        query_pairs_mut.append_pair("token", &self.token);
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct TokenValidationResponse {
    #[serde(default, deserialize_with = "deserialize_u64_or_string")]
    pub client_id: u64,
    #[serde(default, deserialize_with = "deserialize_u64_or_string")]
    pub user_id: u64,
    pub blog_id: Option<u64>,
    pub scope: String,
}

/// Parameters for exchanging an authorization code for an access token.
///
/// After receiving an authorization code via the OAuth2 callback, use these parameters
/// to make a POST request to the token endpoint (`https://public-api.wordpress.com/oauth2/token`)
/// to obtain an access token.
///
/// # Fields
///
/// * `client_id` - Your application's client ID
/// * `client_secret` - Your application's client secret
/// * `code` - The authorization code received from the OAuth2 callback
/// * `grant_type` - Must be `"authorization_code"` for the Authorization Code flow
/// * `redirect_uri` - Must match the redirect URI used in the authorization request
#[derive(Debug, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct TokenRequestParameters {
    pub client_id: u64,
    pub client_secret: String,
    pub code: String,
    #[uniffi(default = "authorization_code")]
    pub grant_type: String,
    pub redirect_uri: String,
}

/// Response from a successful OAuth2 token exchange.
///
/// Returned by the token endpoint after successfully exchanging an authorization code
/// for an access token.
///
/// # Fields
///
/// * `access_token` - The access token to use for authenticated API requests
/// * `token_type` - The token type, typically `"bearer"`
/// * `blog_id` - The ID of the blog the token is authorized for
/// * `blog_url` - The URL of the blog the token is authorized for
/// * `scope` - The granted scope of permissions
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct TokenRequestResponse {
    pub access_token: String,
    pub token_type: String,
    #[serde(
        default,
        deserialize_with = "deserialize_u64_or_none_with_zero_as_none_from_string"
    )]
    pub blog_id: Option<u64>,
    pub blog_url: Option<String>,
    pub scope: String,
}

/// Builds a WordPress.com OAuth2 authorization URL.
///
/// This URL should be opened in a browser to allow the user to authorize your application.
/// After authorization, the user will be redirected to your `redirect_uri` with an
/// authorization code (or token, depending on `response_type`).
///
/// # Arguments
///
/// * `client_id` - Your application's client ID from the WordPress.com developer portal
/// * `redirect_uri` - The URL where users will be redirected after authorization.
///   Must exactly match the URL registered with your application.
/// * `response_type` - The type of OAuth2 flow: `"code"` for Authorization Code flow
///   or `"token"` for Implicit flow
/// * `scope` - Space-separated list of permissions (e.g., `"posts media"`) or `"global"`
///   for full access
/// * `state` - A random string for CSRF protection. Should be verified when the user
///   is redirected back.
/// * `blog` - Optional blog ID to request access to a specific blog
///
/// # Returns
///
/// A `ParsedUrl` containing the authorization URL to open in the user's browser.
#[uniffi::export]
pub fn build_token_request_url(
    client_id: u64,
    redirect_uri: &str,
    scope: &str,
    state: &str,
    blog: Option<u64>,
) -> ParsedUrl {
    let mut url = Url::parse("https://public-api.wordpress.com/oauth2/authorize")
        .expect("Failed to parse url");

    {
        url.query_pairs_mut()
            .append_pair("client_id", &client_id.to_string())
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", scope)
            .append_pair("state", state);

        if let Some(blog_id) = blog {
            url.query_pairs_mut()
                .append_pair("blog", &blog_id.to_string());
        }
    }

    ParsedUrl::new(url)
}

/// Parses the authorization code from an OAuth2 callback URL.
///
/// After the user authorizes your application, WordPress.com redirects them back to your
/// `redirect_uri` with an authorization code in the query parameters. This function extracts
/// that code from the callback URL.
///
/// # Arguments
///
/// * `response` - The full callback URL received after user authorization
///   (e.g., `https://yourapp.com/callback?code=abc123&state=xyz789`)
///
/// # Returns
///
/// * `Ok(String)` - The extracted authorization code
/// * `Err(AuthorizationCodeExtractionError::InvalidUrl)` - If the URL cannot be parsed
/// * `Err(AuthorizationCodeExtractionError::MissingCode)` - If the `code` parameter is not present
#[uniffi::export]
pub fn parse_authorization_url(
    response: String,
) -> Result<AuthorizationCodeExtractionResult, AuthorizationCodeExtractionError> {
    let url = Url::parse(&response)?;

    if let Some(error_code) = value_from_query_pairs("error", &url)
        && let Some(error_description) = value_from_query_pairs("error_description", &url)
    {
        return Err(AuthorizationCodeExtractionError::Error {
            reason: error_description.clone(),
            code: error_code.clone(),
        });
    }

    let state = value_from_query_pairs("state", &url);

    if let Some(code) = value_from_query_pairs("code", &url) {
        Ok(AuthorizationCodeExtractionResult { code, state })
    } else {
        Err(AuthorizationCodeExtractionError::MissingCode)
    }
}

fn value_from_query_pairs(key: &str, url: &Url) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, value)| value.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::io::Read;

    #[test]
    fn test_token_validation_parameters_append_query_pairs() {
        let mut url = Url::parse("https://public-api.wordpress.com/oauth2/token-info")
            .expect("Failed to parse url");

        let params = TokenValidationParameters {
            client_id: 11,
            token: "test_token".to_string(),
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/oauth2/token-info?client_id=11&token=test_token"
        );
    }

    #[test]
    fn test_token_validation_response_deserialization() {
        let json_str =
            r#"{"client_id":"11","user_id":"1234567890","blog_id":null,"scope":"global"}"#;
        let response: TokenValidationResponse = serde_json::from_str(json_str).unwrap();

        assert_eq!(response.client_id, 11);
        assert_eq!(response.user_id, 1234567890);
        assert_eq!(response.blog_id, None);
        assert_eq!(response.scope, "global");
    }

    #[test]
    fn test_token_validation_error_response() {
        let json_str =
            r#"{"error":"invalid_request","error_description":"The specified token is invalid."}"#;
        let result = serde_json::from_str::<TokenValidationResponse>(json_str);

        assert!(result.is_err());
    }

    #[rstest]
    #[case(
        "token-response-01.json",
        "B56F74C1-541D-48D9-8165-6ED3BCD41988",
        Some(123456),
        ""
    )]
    #[case(
        "token-response-02.json",
        "E594436D-1C69-4F55-B32A-5A88DAC68CA6",
        None,
        "global"
    )]
    fn test_token_request_response_parse(
        #[case] json_file_path: &str,
        #[case] token: &str,
        #[case] blog_id: Option<u64>,
        #[case] scope: &str,
    ) {
        let json = test_json(json_file_path).expect("Failed to read JSON file");
        let response: TokenRequestResponse = serde_json::from_slice(json.as_slice()).unwrap();

        assert_eq!(response.access_token, token);
        assert_eq!(response.token_type, "bearer");
        assert_eq!(response.blog_id, blog_id);
        assert_eq!(response.scope, scope);
    }

    /// Tests the Authorization Code Flow URL generation.
    /// Based on: https://developer.wordpress.com/docs/api/oauth2/#OAuth2-Workflows
    #[test]
    fn test_build_token_request_url_authorization_code_flow() {
        let url = build_token_request_url(
            12345,
            "https://yourapp.com/callback",
            "posts media",
            "abc123xyz",
            None,
        );

        assert_eq!(
            url.url(),
            "https://public-api.wordpress.com/oauth2/authorize?client_id=12345&redirect_uri=https%3A%2F%2Fyourapp.com%2Fcallback&response_type=code&scope=posts+media&state=abc123xyz"
        );
    }

    /// Tests URL generation with a specific blog ID.
    /// Based on: https://developer.wordpress.com/docs/api/oauth2/#OAuth2-Workflows
    #[test]
    fn test_build_token_request_url_with_blog() {
        let url = build_token_request_url(
            12345,
            "https://yourapp.com/callback",
            "posts media",
            "abc123xyz",
            Some(67890),
        );

        assert_eq!(
            url.url(),
            "https://public-api.wordpress.com/oauth2/authorize?client_id=12345&redirect_uri=https%3A%2F%2Fyourapp.com%2Fcallback&response_type=code&scope=posts+media&state=abc123xyz&blog=67890"
        );
    }

    /// Tests URL generation with the global scope.
    /// Based on: https://developer.wordpress.com/docs/api/oauth2/#OAuth2-Workflows
    #[test]
    fn test_build_token_request_url_global_scope() {
        let url = build_token_request_url(
            12345,
            "https://yourapp.com/callback",
            "global",
            "abc123xyz",
            None,
        );

        assert_eq!(
            url.url(),
            "https://public-api.wordpress.com/oauth2/authorize?client_id=12345&redirect_uri=https%3A%2F%2Fyourapp.com%2Fcallback&response_type=code&scope=global&state=abc123xyz"
        );
    }

    #[test]
    fn test_extract_code_from_authorization_url_success() {
        let url = "https://yourapp.com/callback?code=abc123&state=xyz789".to_string();
        let result = parse_authorization_url(url);

        assert_eq!(result.expect("Result should be Ok").code, "abc123");
    }

    #[test]
    fn test_extract_code_from_authorization_url_with_other_params() {
        let url = "https://yourapp.com/callback?state=xyz789&code=secret_code&foo=bar".to_string();
        let result = parse_authorization_url(url);

        assert_eq!(result.unwrap().code, "secret_code");
    }

    #[test]
    fn test_extract_code_from_authorization_url_missing_code() {
        let url = "https://yourapp.com/callback?state=xyz789".to_string();
        let result = parse_authorization_url(url);

        assert!(matches!(
            result,
            Err(AuthorizationCodeExtractionError::MissingCode)
        ));
    }

    #[test]
    fn test_extract_code_from_authorization_url_invalid_url() {
        let url = "not a valid url".to_string();
        let result = parse_authorization_url(url);

        assert!(matches!(
            result,
            Err(AuthorizationCodeExtractionError::InvalidUrl { .. })
        ));
    }

    #[test]
    fn test_parse_authorization_url_with_error() {
        let url = "https://yourapp.com/callback?error=access_denied&error_description=The+user+denied+the+request".to_string();
        let result = parse_authorization_url(url);

        match result {
            Err(AuthorizationCodeExtractionError::Error { reason, code }) => {
                assert_eq!(code, "access_denied");
                assert_eq!(reason, "The user denied the request");
            }
            _ => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn test_parse_authorization_url_with_error_only_code() {
        // When error is present but error_description is missing, it should still be MissingCode
        // because the error handling only triggers when both are present
        let url = "https://yourapp.com/callback?error=access_denied".to_string();
        let result = parse_authorization_url(url);

        assert!(matches!(
            result,
            Err(AuthorizationCodeExtractionError::MissingCode)
        ));
    }

    #[test]
    fn test_parse_authorization_url_extracts_state() {
        let url = "https://yourapp.com/callback?code=abc123&state=my_state_value".to_string();
        let result = parse_authorization_url(url).expect("Should succeed");

        assert_eq!(result.code, "abc123");
        assert_eq!(result.state, Some("my_state_value".to_string()));
    }

    #[test]
    fn test_parse_authorization_url_without_state() {
        let url = "https://yourapp.com/callback?code=abc123".to_string();
        let result = parse_authorization_url(url).expect("Should succeed");

        assert_eq!(result.code, "abc123");
        assert_eq!(result.state, None);
    }

    #[test]
    fn test_token_request_parameters_form_urlencoded_serialization() {
        let params = TokenRequestParameters {
            client_id: 12345,
            client_secret: "my_secret".to_string(),
            code: "auth_code_123".to_string(),
            grant_type: "authorization_code".to_string(),
            redirect_uri: "https://example.com/callback".to_string(),
        };

        let encoded = serde_urlencoded::to_string(&params).unwrap();

        assert!(encoded.contains("client_id=12345"));
        assert!(encoded.contains("client_secret=my_secret"));
        assert!(encoded.contains("code=auth_code_123"));
        assert!(encoded.contains("grant_type=authorization_code"));
        assert!(encoded.contains("redirect_uri=https%3A%2F%2Fexample.com%2Fcallback"));
    }

    fn test_json(input: &str) -> Result<Vec<u8>, std::io::Error> {
        let mut file_path = std::path::PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
        file_path.push("wp_api");
        file_path.push("tests");
        file_path.push("wpcom");
        file_path.push("oauth2");
        file_path.push(input);

        let mut f = std::fs::File::open(file_path)?;
        let mut buffer = Vec::new();

        // read the whole file
        f.read_to_end(&mut buffer)?;

        Ok(buffer)
    }
}
