use reusable_test_cases::list_users_cases;
use wp_api::{
    request::endpoint::users_endpoint::UsersRequestListWithEditContextResponse,
    users::{
        UserListParams, WpApiParamUsersHasPublishedPosts, WpApiParamUsersOrderBy,
        WpApiParamUsersWho,
    },
};
use wp_api_integration_tests::prelude::*;

pub mod reusable_test_cases;

#[apply(list_users_cases)]
#[tokio::test]
#[parallel]
async fn list_users_with_edit_context(#[case] params: UserListParams) {
    let request_executor = ReqwestRequestExecutor::default();

    let request_builder = WpApiRequestBuilder::new(
        test_site_api_url_resolver(),
        Arc::new(WpAuthenticationProvider::static_with_username_and_password(
            TestCredentials::instance().admin_username.to_string(),
            TestCredentials::instance().admin_password.to_string(),
        )),
    );
    let wp_request = request_builder.users().list_with_edit_context(&params);
    let response = request_executor.async_request(wp_request.into()).await;
    let result: Result<UsersRequestListWithEditContextResponse, WpApiError> =
        response.unwrap().parse();
    assert!(result.is_ok(), "Response was: '{result:?}'");
}
