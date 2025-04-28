use reusable_test_cases::list_users_cases;
use rstest::*;
use rstest_reuse::{self, apply};
use serial_test::parallel;
use std::sync::Arc;
use wp_api::{
    WpApiError, WpApiParamOrder, WpApiRequestBuilder,
    auth::WpAuthenticationProvider,
    generate,
    request::endpoint::users_endpoint::UsersRequestListWithEditContextResponse,
    reqwest_request_executor::ReqwestRequestExecutor,
    users::{
        UserListParams, WpApiParamUsersHasPublishedPosts, WpApiParamUsersOrderBy,
        WpApiParamUsersWho,
    },
};
use wp_api_integration_tests::{FIRST_USER_ID, SECOND_USER_ID, TestCredentials, test_site_url};

pub mod reusable_test_cases;

#[apply(list_users_cases)]
#[tokio::test]
#[parallel]
async fn list_users_with_edit_context(#[case] params: UserListParams) {
    let request_executor = ReqwestRequestExecutor::default();

    let request_builder = WpApiRequestBuilder::new(
        test_site_url(),
        Arc::new(WpAuthenticationProvider::static_with_username_and_password(
            TestCredentials::instance().admin_username.to_string(),
            TestCredentials::instance().admin_password.to_string(),
        )),
    );
    let wp_request = request_builder.users().list_with_edit_context(&params);
    let response = request_executor.async_request(wp_request.into()).await;
    let result: Result<UsersRequestListWithEditContextResponse, WpApiError> =
        response.unwrap().parse();
    assert!(result.is_ok(), "Response was: '{:?}'", result);
}
