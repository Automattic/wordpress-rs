use reusable_test_cases::list_users_cases;
use rstest::*;
use rstest_reuse::{self, apply};
use serial_test::parallel;
use wp_api::{
    generate,
    users::{
        UserListParams, WpApiParamUsersHasPublishedPosts, WpApiParamUsersOrderBy,
        WpApiParamUsersWho,
    },
    WpApiParamOrder, WpApiRequestBuilder, WpAuthentication,
};
use wp_api_integration_tests::{
    test_site_url, AsyncWpNetworking, TestCredentials, FIRST_USER_ID, SECOND_USER_ID,
};

pub mod reusable_test_cases;

#[apply(list_users_cases)]
#[tokio::test]
#[parallel]
async fn list_users_with_edit_context(#[case] params: UserListParams) {
    use wp_api::{
        request::endpoint::users_endpoint::UsersRequestListWithEditContextResponse, WpApiError,
    };

    let authentication = WpAuthentication::from_username_and_password(
        TestCredentials::instance().admin_username.to_string(),
        TestCredentials::instance().admin_password.to_string(),
    );
    let async_wp_networking = AsyncWpNetworking::default();

    let request_builder = WpApiRequestBuilder::new(test_site_url(), authentication);
    let wp_request = request_builder.users().list_with_edit_context(&params);
    let response = async_wp_networking.async_request(wp_request.into()).await;
    let result: Result<UsersRequestListWithEditContextResponse, WpApiError> =
        response.unwrap().parse();
    assert!(result.is_ok(), "Response was: '{:?}'", result);
}
