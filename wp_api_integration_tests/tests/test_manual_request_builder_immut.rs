use reusable_test_cases::list_users_cases;
use rstest::*;
use rstest_reuse::{self, apply};
use serial_test::parallel;
use wp_api::authenticator::{ApplicationPasswordAuthenticator, Authenticator};
use wp_api::{
    generate,
    users::{
        UserListParams, UserWithEditContext, WpApiParamUsersHasPublishedPosts,
        WpApiParamUsersOrderBy, WpApiParamUsersWho,
    },
    WpApiError, WpApiParamOrder, WpApiRequestBuilder,
};
use wp_api_integration_tests::{
    test_site_url, AsyncWpNetworking, TestCredentials, FIRST_USER_ID, SECOND_USER_ID,
};

pub mod reusable_test_cases;

#[apply(list_users_cases)]
#[tokio::test]
#[parallel]
async fn list_users_with_edit_context(#[case] params: UserListParams) {
    let authenticator = ApplicationPasswordAuthenticator::with_application_password(
        test_site_url().inner.host_str().unwrap_or("").to_string(),
        TestCredentials::instance().admin_username.to_string(),
        TestCredentials::instance().admin_password.to_string(),
    );
    let async_wp_networking = AsyncWpNetworking::default();

    let request_builder = WpApiRequestBuilder::new(test_site_url());
    let mut wp_request = request_builder.users().list_with_edit_context(&params);
    if let Some(headers) = authenticator.authentication_headers().await {
        wp_request.add_headers(&headers);
    }
    let response = async_wp_networking.async_request(wp_request.into()).await;
    let result = response
        .unwrap()
        .parse::<Vec<UserWithEditContext>, WpApiError>();
    assert!(result.is_ok(), "Response was: '{:?}'", result);
}
