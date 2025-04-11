use mock_request_executor::MockRequestExecutor;
use serial_test::parallel;
use std::{collections::HashMap, path::Path, sync::Arc};
use wp_api::{
    InvalidSslErrorReason, RequestExecutionError, RequestExecutionErrorReason,
    login::{
        login_client::WpLoginClient,
        url_discovery::{
            ApplicationPasswordsNotSupportedReason, AutoDiscoveryAttemptFailure,
            FetchAndParseApiRootFailure, FindApiRootFailure,
        },
    },
    middleware::{ApiDiscoveryAuthenticationMiddleware, WpApiMiddleware, WpApiMiddlewarePipeline},
    request::RequestExecutor,
    reqwest_request_executor::ReqwestRequestExecutor,
};

#[tokio::test]
#[parallel]
async fn login_spec_1_valid_site_works_correctly() {
    // Spec Example 1
    assert_eq!(
        login_url("https://vanilla.wpmt.co").await,
        "https://vanilla.wpmt.co/wp-admin/authorize-application.php"
    );
}

#[tokio::test]
#[parallel]
async fn login_spec_2_local_development_environment() {
    // Spec Example 2
    let mut mocks = HashMap::new();
    mocks.insert(
        "http://localhost/".to_string(),
        response_helpers::with_api_root("http://localhost/wp-json"),
    );
    mocks.insert(
        "https://localhost/".to_string(),
        response_helpers::with_api_root("http://localhost/wp-json"),
    );
    mocks.insert(
        "http://localhost/wp-json".to_string(),
        response_helpers::json_response_from_path(Path::new(
            "../native/swift/Tests/wordpress-api/Resources/Responses/localhost-json-root.json",
        )),
    );
    mocks.insert(
        "https://localhost/wp-json".to_string(),
        response_helpers::json_response_from_path(Path::new(
            "../native/swift/Tests/wordpress-api/Resources/Responses/localhost-json-root.json",
        )),
    );
    let request_executor = Arc::new(MockRequestExecutor::new(mocks));
    let error = discovery_helper(request_executor, vec![], "http://localhost/")
        .await
        .expect_err("Expected api discovery to fail")
        .to_fetch_and_parse_api_root_failure();
    if let FetchAndParseApiRootFailure::ApplicationPasswordsNotSupported { reason, .. } = error {
        assert_eq!(
            reason,
            Some(ApplicationPasswordsNotSupportedReason::SiteIsLocalDevelopmentEnvironment)
        );
    } else {
        panic!(
            "Expected FetchAndParseApiRootFailure::ApplicationPasswordsNotSupported, got: {:?}",
            error
        );
    }
}

#[tokio::test]
#[parallel]
async fn login_spec_3_admin_url_provided() {
    // Spec Example 3
    assert_eq!(
        login_url("https://vanilla.wpmt.co/wp-login.php").await,
        "https://vanilla.wpmt.co/wp-admin/authorize-application.php"
    );
    assert_eq!(
        login_url("https://vanilla.wpmt.co/wp-admin").await,
        "https://vanilla.wpmt.co/wp-admin/authorize-application.php"
    );
}

#[tokio::test]
#[parallel]
async fn login_spec_4_auth_https_support() {
    // Spec Example 4
    assert_eq!(
        login_url("http://vanilla.wpmt.co").await,
        "https://vanilla.wpmt.co/wp-admin/authorize-application.php"
    );
}

#[tokio::test]
#[parallel]
async fn login_spec_5_http_only_site() {
    // Spec Example 5
    let error = login_err("http://no-https.wpmt.co")
        .await
        .to_fetch_and_parse_api_root_failure();
    if let FetchAndParseApiRootFailure::ApplicationPasswordsNotSupported { reason, .. } = error {
        assert_eq!(
            reason,
            Some(ApplicationPasswordsNotSupportedReason::ApplicationPasswordsDisabledForHttpSite)
        );
    } else {
        panic!(
            "Expected FetchAndParseApiRootFailure::ApplicationPasswordsNotSupported, got: {:?}",
            error
        );
    }
}

#[tokio::test]
#[parallel]
async fn login_spec_6_http_only_site_with_application_passwords_enabled() {
    // Spec Example 6
    assert_eq!(
        login_url("http://no-https-with-application-passwords.wpmt.co").await,
        "http://no-https-with-application-passwords.wpmt.co/wp-admin/authorize-application.php"
    );
}

#[tokio::test]
#[parallel]
async fn login_spec_7_aggressively_cached_site_with_no_link_header() {
    // Spec Example 7
    assert_eq!(
        login_url("https://aggressive-caching.wpmt.co").await,
        "https://aggressive-caching.wpmt.co/wp-admin/authorize-application.php"
    );
}

#[tokio::test]
#[parallel]
async fn login_spec_8_site_with_application_passwords_disabled_by_wordfence() {
    // Spec Example 8
    let error = login_err("https://wordfence.wpmt.co")
        .await
        .to_fetch_and_parse_api_root_failure();
    if let FetchAndParseApiRootFailure::ApplicationPasswordsNotSupported {
        reason:
            Some(ApplicationPasswordsNotSupportedReason::ApplicationPasswordBlockedByPlugin {
                ref plugin,
            }),
        ..
    } = error
    {
        assert_eq!(plugin.name, "Wordfence");
    } else {
        panic!(
            "Expected ApplicationPasswordsNotSupportedReason::ApplicationPasswordBlockedByPlugin, got: {:?}",
            error
        );
    }
}

#[tokio::test]
#[parallel]
async fn login_spec_9_not_a_wordpress_site() {
    // Spec Example 9
    assert_eq!(
        login_err("google.com").await.to_find_api_root_failure(),
        FindApiRootFailure::ProbablyNotAWordPressSite
    );
}

#[tokio::test]
#[parallel]
async fn login_spec_10_wordpress_subdirectory_with_link_header() {
    // Spec Example 10
    assert_eq!(
        login_url("https://subdirectory.wpmt.co/index.php?link_header=true").await,
        "https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php"
    );
}

#[tokio::test]
#[parallel]
async fn login_spec_11_wordpress_subdirectory_with_link_tag() {
    // Spec Example 11
    assert_eq!(
        login_url("https://subdirectory.wpmt.co/index.php?link_tag=true").await,
        "https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php"
    );
}

#[tokio::test]
#[parallel]
async fn login_spec_12_wordpress_subdirectory_with_redirect() {
    // Spec Example 12
    assert_eq!(
        login_url("https://subdirectory.wpmt.co/index.php?redirect=true").await,
        "https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php"
    );
}

#[tokio::test]
#[parallel]
async fn login_spec_13_wordpress_http_basic_with_missing_credentials() {
    // Spec Example 13 (with missing credentials)
    let expected_hostname = "https://basic-auth.wpmt.co/";
    let reason = login_err(expected_hostname)
        .await
        .to_fetch_home_page_reason();
    if let RequestExecutionErrorReason::HttpAuthenticationRequiredError { hostname, .. } = reason {
        assert_eq!(hostname, expected_hostname);
    } else {
        panic!(
            "Expected RequestExecutionErrorReason::HttpAuthenticationRequiredError, got: {:?}",
            reason
        );
    }
}

#[tokio::test]
#[parallel]
async fn login_spec_13_wordpress_http_basic_with_invalid_credentials() {
    // Spec Example 13 (with invalid credentials)
    let expected_hostname = "https://basic-auth.wpmt.co/";
    let reason = discovery_helper(
        Arc::new(ReqwestRequestExecutor::default()),
        vec![Arc::new(ApiDiscoveryAuthenticationMiddleware::new(
            "invalid".to_string(),
            "invalid".to_string(),
        ))],
        expected_hostname,
    )
    .await
    .expect_err("Expected api discovery to fail")
    .to_fetch_home_page_reason();
    if let RequestExecutionErrorReason::HttpAuthenticationRejectedError { hostname, .. } = reason {
        assert_eq!(hostname, expected_hostname);
    } else {
        panic!(
            "Expected RequestExecutionErrorReason::HttpAuthenticationRejectedError, got: {:?}",
            reason
        );
    }
}

#[tokio::test]
#[parallel]
async fn login_spec_13_wordpress_http_basic_with_valid_credentials() {
    // Spec Example 13 (with valid credentials)
    let login_url = discovery_helper(
        Arc::new(ReqwestRequestExecutor::default()),
        vec![Arc::new(ApiDiscoveryAuthenticationMiddleware::new(
            "test@example.com".to_string(),
            "str0ngp4ssw0rd!".to_string(),
        ))],
        "https://basic-auth.wpmt.co/",
    )
    .await
    .expect("Expected api discovery to fail");
    assert_eq!(
        login_url,
        "https://basic-auth.wpmt.co/wp-admin/authorize-application.php"
    );
}

#[tokio::test]
#[parallel]
async fn login_spec_14_wordpress_custom_rest_api_prefix() {
    // Spec Example 14
    assert_eq!(
        login_url("https://custom-rest-prefix.wpmt.co").await,
        "https://custom-rest-prefix.wpmt.co/wp-admin/authorize-application.php"
    );
}

#[tokio::test]
#[parallel]
async fn login_spec_15_wordpress_heavy_rate_limiting() {
    // Spec Example 15
    assert_eq!(
        login_url("https://aggressive-rate-limiting.wpmt.co").await,
        "https://aggressive-rate-limiting.wpmt.co/wp-admin/authorize-application.php"
    );
}
//
//#[tokio::test]
//#[parallel]
//async fn login_spec_15_wordpress_heavy_rate_limiting_that_never_succeeds() {
//    todo!("We need mocking for this");
//    // Spec Example 15
//}

#[tokio::test]
#[parallel]
async fn login_spec_16_invalid_url() {
    // Spec Example 16
    assert!(matches!(
        login_err("https://valid-looking-url-but-not-actually.foo")
            .await
            .to_find_api_root_failure(),
        FindApiRootFailure::FetchHomepage { .. }
    ));
}

#[tokio::test]
#[parallel]
async fn login_spec_17_invalid_https_fails() {
    // Spec Example 17
    let request_execution_error_reason =
        login_err("https://wordpress-1315525-4803651.cloudwaysapps.com")
            .await
            .to_fetch_home_page_reason();
    assert!(
        matches!(
            request_execution_error_reason,
            RequestExecutionErrorReason::InvalidSslError {
                reason: InvalidSslErrorReason::CertificateNotValidForName { .. }
            }
        ),
        "Expected RequestExecutionErrorReason::InvalidSslError, got: {:?}",
        request_execution_error_reason
    );
}

#[tokio::test]
#[parallel]
async fn login_spec_17_invalid_https_with_exception_works() {
    // Spec Example 17 (with exception)
    assert_eq!(
        login_url_with_executor(
            Arc::new(ReqwestRequestExecutor::new_with_default_timeout(true)),
            "https://wordpress-1315525-4803651.cloudwaysapps.com"
        )
        .await,
        "https://vanilla.wpmt.co/wp-admin/authorize-application.php"
    );
}

async fn login_url(site_url: &str) -> String {
    login_url_with_executor(Arc::new(ReqwestRequestExecutor::default()), site_url).await
}

async fn login_url_with_executor(
    request_executor: Arc<dyn RequestExecutor>,
    site_url: &str,
) -> String {
    discovery_helper(request_executor, vec![], site_url)
        .await
        .expect("Expected api discovery to be successful")
}

trait AutoDiscoveryAttemptFailureExtension {
    fn to_find_api_root_failure(self) -> FindApiRootFailure;
    fn to_fetch_home_page_reason(self) -> RequestExecutionErrorReason;
    fn to_fetch_and_parse_api_root_failure(self) -> FetchAndParseApiRootFailure;
}

impl AutoDiscoveryAttemptFailureExtension for AutoDiscoveryAttemptFailure {
    fn to_find_api_root_failure(self) -> FindApiRootFailure {
        if let AutoDiscoveryAttemptFailure::FindApiRoot {
            find_api_root_failure,
            ..
        } = self
        {
            find_api_root_failure.clone()
        } else {
            panic!(
                "Expected AutoDiscoveryAttemptFailure::FindApiRoot, got: {:?}",
                self
            );
        }
    }

    fn to_fetch_home_page_reason(self) -> RequestExecutionErrorReason {
        let error = self.to_find_api_root_failure();
        if let FindApiRootFailure::FetchHomepage {
            error: RequestExecutionError::RequestExecutionFailed { reason, .. },
        } = error
        {
            reason
        } else {
            panic!(
                "Expected FindApiRootFailure::FetchHomepage, got: {:?}",
                error
            );
        }
    }

    fn to_fetch_and_parse_api_root_failure(self) -> FetchAndParseApiRootFailure {
        if let AutoDiscoveryAttemptFailure::FetchAndParseApiRoot {
            fetch_and_parse_api_root_failure,
            ..
        } = self
        {
            fetch_and_parse_api_root_failure
        } else {
            panic!(
                "Expected AutoDiscoveryAttemptFailure::FetchAndParseApiRoot, got: {:?}",
                self
            );
        }
    }
}

async fn login_err(site_url: &str) -> AutoDiscoveryAttemptFailure {
    discovery_helper(
        Arc::new(ReqwestRequestExecutor::default()),
        vec![],
        site_url,
    )
    .await
    .expect_err("Expected api discovery to fail")
}

async fn discovery_helper(
    request_executor: Arc<dyn RequestExecutor>,
    middlewares: Vec<Arc<dyn WpApiMiddleware>>,
    site_url: &str,
) -> Result<String, AutoDiscoveryAttemptFailure> {
    let client = WpLoginClient::new(
        request_executor,
        Arc::new(WpApiMiddlewarePipeline { middlewares }),
    );
    client
        .api_discovery(site_url.to_string())
        .await
        .combined_result()
        .map(|success| {
            success
                .api_details
                .find_application_passwords_authentication_url()
                .expect("If the discovery is successful, authentication url has to be `Some`")
        })
        .map_err(|e| e.clone())
}

mod mock_request_executor {
    use std::{collections::HashMap, sync::Arc};
    use wp_api::{
        MediaUploadRequestExecutionError, RequestExecutionError,
        request::{
            RequestExecutor, WpNetworkRequest, WpNetworkResponse,
            endpoint::media_endpoint::MediaUploadRequest,
        },
    };

    #[derive(Debug)]
    pub struct MockRequestExecutor {
        mocks: HashMap<String, WpNetworkResponse>,
    }

    impl MockRequestExecutor {
        pub fn new(mocks: HashMap<String, WpNetworkResponse>) -> Self {
            Self { mocks }
        }
    }

    #[async_trait::async_trait]
    impl RequestExecutor for MockRequestExecutor {
        async fn execute(
            &self,
            request: Arc<WpNetworkRequest>,
        ) -> Result<WpNetworkResponse, RequestExecutionError> {
            let response = self
                .mocks
                .iter()
                .find_map(|(url, response)| {
                    if url == &request.url().0 {
                        Some(response)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    panic!(
                        "No mock response found for the request URL: {}",
                        request.url().0
                    )
                });
            // `WpNetworkResponse` doesn't implement `Clone` because it may be expensive, but we
            // can clone it manually when it makes sense
            Ok(WpNetworkResponse {
                body: response.body.clone(),
                status_code: response.status_code,
                response_header_map: response.response_header_map.clone(),
                request_url: response.request_url.clone(),
                request_header_map: response.request_header_map.clone(),
            })
        }

        async fn upload_media(
            &self,
            _: Arc<MediaUploadRequest>,
        ) -> Result<WpNetworkResponse, MediaUploadRequestExecutionError> {
            panic!("Upload media is not necessary for these tests");
        }

        async fn sleep(&self, _: u64) {}
    }
}

mod response_helpers {
    use http::{HeaderMap, header::HeaderValue};
    use std::{fs, path::Path, sync::Arc};
    use wp_api::request::{WpNetworkHeaderMap, WpNetworkResponse, endpoint::WpEndpointUrl};

    pub fn with_api_root(url: &str) -> WpNetworkResponse {
        let mut map = HeaderMap::new();
        let link_header_value = format!("<{url}>; rel=\"https://api.w.org/\"");
        map.insert(
            http::header::LINK,
            HeaderValue::from_str(&link_header_value).expect("Failed to create Link header"),
        );
        WpNetworkResponse {
            body: vec![],
            status_code: 200,
            response_header_map: Arc::new(map.into()),
            request_url: WpEndpointUrl("".to_string()),
            request_header_map: WpNetworkHeaderMap::default().into(),
        }
    }

    pub fn json_response_from_path(json_file_path: &Path) -> WpNetworkResponse {
        let json = fs::read_to_string(json_file_path).unwrap_or_else(|_| {
            panic!(
                "Should have been able to read the json file at: '{:#?}'",
                json_file_path
            )
        });
        let mut map = HeaderMap::new();
        map.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        WpNetworkResponse {
            body: json.as_bytes().to_vec(),
            status_code: 200,
            response_header_map: Arc::new(map.into()),
            request_url: WpEndpointUrl("".to_string()),
            request_header_map: WpNetworkHeaderMap::default().into(),
        }
    }

    pub fn retry_response(delay: usize) -> WpNetworkResponse {
        let mut map = HeaderMap::new();
        map.insert(
            http::header::RETRY_AFTER,
            HeaderValue::from_str(format!("{delay}").as_str())
                .expect("Failed to create Retry-After header"),
        );
        WpNetworkResponse {
            body: vec![],
            status_code: 429,
            response_header_map: Arc::new(map.into()),
            request_url: WpEndpointUrl("".to_string()),
            request_header_map: WpNetworkHeaderMap::default().into(),
        }
    }
}
