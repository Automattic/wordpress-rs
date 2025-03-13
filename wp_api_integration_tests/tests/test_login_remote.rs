use serial_test::parallel;
use std::any::Any;
use std::sync::Arc;
use wp_api::{login::login_client::WpLoginClient, request_executor::WpRequestExecutor, WpApiError};

#[tokio::test]
#[parallel]
async fn test_valid_site_works_correctly() {
    // Spec Example 1
    let result = login_url("https://vanilla.wpmt.co").await;
    assert_eq!(
        result,
        Ok("https://vanilla.wpmt.co/wp-admin/authorize-application.php".to_string())
    );
}

#[tokio::test]
#[parallel]
async fn test_local_development_environment() {
    // Spec Example 2

    // Until we have a mock server, this is just testing that we can examine underlying errors
    let result = login_url("http://localhost").await;
    let error = result.unwrap_err();
    assert!(is_wp_api_error(&error));
    assert_eq!(
        error.to_string(),
        "A server with the specified hostname could not be found."
    );
}

#[tokio::test]
#[parallel]
async fn test_admin_url_provided() {
    // Spec Example 3
    assert_eq!(
        login_url("https://vanilla.wpmt.co/wp-login.php").await,
        Ok("https://vanilla.wpmt.co/wp-admin/authorize-application.php".to_string())
    );
    assert_eq!(
        login_url("https://vanilla.wpmt.co/wp-admin").await,
        Ok("https://vanilla.wpmt.co/wp-admin/authorize-application.php".to_string())
    );
}

#[tokio::test]
#[parallel]
async fn test_auth_https_support() {
    // Spec Example 4
    assert_eq!(
        login_url("http://vanilla.wpmt.co").await,
        Ok("https://vanilla.wpmt.co/wp-admin/authorize-application.php".to_string())
    );
}

#[tokio::test]
#[parallel]
async fn test_http_only_site() {
    // Spec Example 5
    let result = login_url("http://no-https.wpmt.co").await;
    let error = result.unwrap_err();
    assert!(is_wp_api_error(&error));
    assert_eq!(error.to_string(), "Application Passwords is not enabled for this site – this is likely because we can't establish a secure connection to it. Please add an SSL certificate to this site and try again.");
}

#[tokio::test]
#[parallel]
async fn test_http_only_site_with_application_passwords_enabled() {
    // Spec Example 6
    assert_eq!(
        login_url("http://no-https-with-application-passwords.wpmt.co").await,
        Ok(
            "http://no-https-with-application-passwords.wpmt.co/wp-admin/authorize-application.php"
                .to_string()
        )
    );
}

#[tokio::test]
#[parallel]
async fn test_aggressively_cached_site_with_no_link_header() {
    // Spec Example 7
    assert_eq!(
        login_url("https://aggressive-caching.wpmt.co").await,
        Ok("https://aggressive-caching.wpmt.co/wp-admin/authorize-application.php".to_string())
    );
}

#[tokio::test]
#[parallel]
async fn test_site_with_application_passwords_disabled_by_wordfence() {
    // Spec Example 8
    let result = login_url("https://wordfence.wpmt.co").await;
    let error = result.unwrap_err();
    assert!(is_wp_api_error(&error));
    assert_eq!(error.to_string(), "Unable to login to https://wordfence.wpmt.co/ – the Wordfence plugin might have disabled Application Passwords. Please visit https://www.wordfence.com/support/ to learn more");
}

#[tokio::test]
#[parallel]
async fn test_not_a_wordpress_site() {
    todo!("We need support for the 'not a wordpress site' error");
    // Spec Example 9
}

#[tokio::test]
#[parallel]
async fn test_wordpress_subdirectory_with_link_header() {
    // Spec Example 10
    assert_eq!(
        login_url("https://subdirectory.wpmt.co/index.php?link_header=true").await,
        Ok("https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php".to_string())
    );
}

#[tokio::test]
#[parallel]
async fn test_wordpress_subdirectory_with_link_tag() {
    // Spec Example 11
    assert_eq!(
        login_url("https://subdirectory.wpmt.co/index.php?link_tag=true").await,
        Ok("https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php".to_string())
    );
}

#[tokio::test]
#[parallel]
async fn test_wordpress_subdirectory_with_redirect() {
    // Spec Example 12
    assert_eq!(
        login_url("https://subdirectory.wpmt.co/index.php?redirect=true").await,
        Ok("https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php".to_string())
    );
}

#[tokio::test]
#[parallel]
async fn test_wordpress_http_basic_with_missing_credentials() {
    todo!("HTTP Basic support isn't working yet");
    // Spec Example 13 (with missing credentials)
}

#[tokio::test]
#[parallel]
async fn test_wordpress_http_basic_with_invalid_credentials() {
    todo!("HTTP Basic support isn't working yet");
    // Spec Example 13 (with invalid credentials)
}

#[tokio::test]
#[parallel]
async fn test_wordpress_http_basic_with_valid_credentials() {
    todo!("HTTP Basic support isn't working yet");
    // Spec Example 13 (with valid credentials)
}

#[tokio::test]
#[parallel]
async fn test_wordpress_custom_rest_api_prefix() {
    // Spec Example 12
    assert_eq!(
        login_url("https://custom-rest-prefix.wpmt.co").await,
        Ok("https://custom-rest-prefix.wpmt.co/wp-admin/authorize-application.php".to_string())
    );
}

#[tokio::test]
#[parallel]
async fn test_wordpress_heavy_rate_limiting() {
    // Spec Example 15
    assert_eq!(
        login_url("https://aggressive-rate-limiting.wpmt.co").await,
        Ok(
            "https://aggressive-rate-limiting.wpmt.co/wp-admin/authorize-application.php"
                .to_string()
        )
    );
}

#[tokio::test]
#[parallel]
async fn test_wordpress_heavy_rate_limiting_that_never_succeeds() {
    todo!("We need mocking for this");
    // Spec Example 15
}

#[tokio::test]
#[parallel]
async fn test_invalid_url() {
    // Spec Example 16
    let result = login_url("https://valid-looking-url-but-not-actually.foo").await;
    let error = result.unwrap_err();
    assert!(is_wp_api_error(&error));
    assert_eq!(error.to_string(), "");
}

#[tokio::test]
#[parallel]
async fn test_invalid_https_fails() {
    // Spec Example 17
    let result = login_url("https://wordpress-1315525-4803651.cloudwaysapps.com").await;
    let error = result.unwrap_err();
    assert!(is_wp_api_error(&error));
    assert_eq!(error.to_string(), "");
}

#[tokio::test]
#[parallel]
async fn test_invalid_https_with_exception_works() {
    // Spec Example 17 (with exception)
    assert_eq!(
        login_url("https://wordpress-1315525-4803651.cloudwaysapps.com").await,
        Ok("https://vanilla.wpmt.co/wp-admin/authorize-application.php".to_string())
    );
}

fn is_wp_api_error(error: &dyn Any) -> bool {
    error.is::<WpApiError>()
}

async fn login_url(site_url: &str) -> Result<String, WpApiError> {
    let executor = WpRequestExecutor::default();
    let client = WpLoginClient::new(Arc::new(executor));
    let result = client.api_discovery(site_url.to_string()).await;

    if let Some(successful_result) = result.find_successful() {
        let api_details = successful_result
            .api_discovery_result
            .clone()
            .expect("Already verified that auto discovery is successful")
            .api_details;
        let application_passwords_authentication_url = api_details
            .find_application_passwords_authentication_url()
            .expect("Already verified that auto discovery is successful");
        return Ok(application_passwords_authentication_url.to_string());
    }

    if let Some(error) = result.user_input_attempt().error() {
        if let Ok(error) = error.clone().try_into() {
            return Err(error);
        } else {
            return Err(WpApiError::RequestExecutionFailed {
                status_code: None,
                redirects: None,
                reason: wp_api::RequestExecutionErrorReason::GenericError {
                    error_message: error.to_string(),
                },
            });
        }
    }

    if let Some(error) = result.auto_discovery_attempt().error() {
        if let Ok(error) = error.try_into() {
            return Err(error);
        }
    }

    return Err(WpApiError::UnknownError {
        status_code: 0,
        response: "Unknown error".to_string(),
    });
}
