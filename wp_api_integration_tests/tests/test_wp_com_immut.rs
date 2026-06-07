use serial_test::parallel;
use wp_api_integration_tests::wp_com_client_with_invalid_token;

#[tokio::test]
#[parallel]
async fn use_invalid_token_get_me() {
    let client = wp_com_client_with_invalid_token();
    let result = client.me().get().await;
    let err = result.unwrap_err();
    let status_code = err.status_code().unwrap();
    assert!(
        (400..500).contains(&status_code),
        "Expected status code in 4xx range, got: {status_code}"
    );
}

#[tokio::test]
#[parallel]
async fn use_invalid_token_get_support_conversation_list() {
    let client = wp_com_client_with_invalid_token();
    let result = client
        .support_tickets()
        .get_support_conversation_list()
        .await;
    let err = result.unwrap_err();
    let status_code = err.status_code().unwrap();
    assert!(
        (400..500).contains(&status_code),
        "Expected status code in 4xx range, got: {status_code}"
    );
}
