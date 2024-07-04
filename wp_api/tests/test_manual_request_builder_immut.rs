use integration_test_common::{
    test_site_url, AsyncWpNetworking, FIRST_USER_ID, SECOND_USER_ID,
    TEST_CREDENTIALS_ADMIN_PASSWORD, TEST_CREDENTIALS_ADMIN_USERNAME,
};
use reusable_test_cases::list_users_cases;
use rstest::*;
use rstest_reuse::{self, apply};
use serial_test::parallel;
use wp_api::{
    generate,
    users::UserWithEditContext,
    users::{
        UserListParams, WpApiParamUsersHasPublishedPosts, WpApiParamUsersOrderBy,
        WpApiParamUsersWho,
    },
    WpApiParamOrder, WpApiRequestBuilder, WpAuthentication,
};

pub mod integration_test_common;
pub mod reusable_test_cases;

#[apply(list_users_cases)]
#[tokio::test]
#[parallel]
async fn list_users_with_edit_context(#[case] params: UserListParams) {
    let authentication = WpAuthentication::from_username_and_password(
        TEST_CREDENTIALS_ADMIN_USERNAME.to_string(),
        TEST_CREDENTIALS_ADMIN_PASSWORD.to_string(),
    );
    let async_wp_networking = AsyncWpNetworking::default();

    let request_builder = WpApiRequestBuilder::new(test_site_url(), authentication);
    let wp_request = request_builder.users().list_with_edit_context(&params);
    let response = async_wp_networking.async_request(wp_request.into()).await;
    let result = response.unwrap().parse::<Vec<UserWithEditContext>>();
    assert!(result.is_ok(), "Response was: '{:?}'", result);
}
