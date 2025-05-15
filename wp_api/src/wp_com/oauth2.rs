use crate::url_query::AppendUrlQueryPairs;
use crate::url_query::QueryPairs;
use serde::{Deserialize, Serialize};
use wp_serde_helper::deserialize_u64_or_string;

#[derive(Debug, Serialize, uniffi::Record)]
pub struct TokenValidationParameters {
    pub client_id: String,
    pub token: String,
}

impl AppendUrlQueryPairs for TokenValidationParameters {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut.append_pair("client_id", &self.client_id);
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

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    #[test]
    fn test_token_validation_parameters_append_query_pairs() {
        let mut url = Url::parse("https://public-api.wordpress.com/oauth2/token-info")
            .expect("Failed to parse url");

        let params = TokenValidationParameters {
            client_id: "11".to_string(),
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
}
