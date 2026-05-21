use wp_api::{
    auth::WpAuthenticationProvider,
    media::{MediaCreateParams, MediaId, MediaListParams, MediaUpdateParams},
    posts::WpApiParamPostsOrderBy,
    prelude::*,
    request::{RequestContext, WpMultipartFormField, WpMultipartFormRequest},
    users::UserId,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn create_media_err_cannot_create() {
    api_client_as_subscriber()
        .media()
        .create(&MediaCreateParams {
            file_path: MEDIA_TEST_FILE_PATH.to_string(),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::CannotCreate)
}

#[tokio::test]
#[parallel]
async fn create_media_err_upload_no_data() {
    api_client_with_medir_err_networking(MediaErrNetworkingTestType::UploadNoData)
        .media()
        .create(&MediaCreateParams {
            file_path: MEDIA_TEST_FILE_PATH.to_string(),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::UploadNoData)
}

#[tokio::test]
#[parallel]
async fn delete_media_err_cannot_delete() {
    api_client_as_subscriber()
        .media()
        .delete(&MEDIA_ID_611)
        .await
        .assert_wp_error(WpErrorCode::CannotDelete);
}

#[tokio::test]
#[parallel]
async fn list_err_no_search_term_defined() {
    api_client()
        .media()
        .list_with_edit_context(&MediaListParams {
            orderby: Some(WpApiParamPostsOrderBy::Relevance),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::NoSearchTermDefined);
}

#[tokio::test]
#[parallel]
async fn list_err_order_by_include_missing_include() {
    api_client()
        .media()
        .list_with_edit_context(&MediaListParams {
            orderby: Some(WpApiParamPostsOrderBy::Include),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::OrderbyIncludeMissingInclude);
}

#[tokio::test]
#[parallel]
async fn list_err_media_invalid_page_number() {
    api_client()
        .media()
        .list_with_edit_context(&MediaListParams {
            page: Some(99999999),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::PostInvalidPageNumber);
}

#[tokio::test]
#[parallel]
async fn retrieve_media_err_forbidden_context() {
    api_client_as_subscriber()
        .media()
        .retrieve_with_edit_context(&MEDIA_ID_611)
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn retrieve_media_err_media_invalid_id() {
    api_client()
        .media()
        .retrieve_with_edit_context(&MediaId(99999999))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

#[tokio::test]
#[parallel]
async fn update_media_err_cannot_edit() {
    api_client_as_author()
        .media()
        .update(&MEDIA_ID_611, &MediaUpdateParams::default())
        .await
        .assert_wp_error(WpErrorCode::CannotEdit);
}

#[tokio::test]
#[parallel]
async fn update_media_err_invalid_author() {
    api_client()
        .media()
        .update(
            &MEDIA_ID_611,
            &MediaUpdateParams {
                author: Some(UserId(99999999)),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::InvalidAuthor);
}

#[tokio::test]
#[parallel]
async fn update_media_err_invalid_template() {
    api_client()
        .media()
        .update(
            &MEDIA_ID_611,
            &MediaUpdateParams {
                template: Some("foo".to_string()),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::InvalidParam);
}

#[tokio::test]
#[parallel]
async fn update_media_err_post_invalid_id() {
    api_client_as_author()
        .media()
        .update(&MediaId(99999999), &MediaUpdateParams::default())
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

fn api_client_with_medir_err_networking(test_type: MediaErrNetworkingTestType) -> WpApiClient {
    WpApiClient::new(
        test_site_api_url_resolver(),
        WpApiClientDelegate {
            auth_provider: Arc::new(WpAuthenticationProvider::static_with_username_and_password(
                TestCredentials::instance().admin_username.to_string(),
                TestCredentials::instance().admin_password.to_string(),
            )),
            request_executor: Arc::new(MediaErrNetworking::new(test_type)),
            middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
            app_notifier: Arc::new(EmptyAppNotifier),
        },
    )
}

#[derive(Debug)]
enum MediaErrNetworkingTestType {
    UploadNoData,
}

#[derive(Debug)]
struct MediaErrNetworking {
    client: reqwest::Client,
    test_type: MediaErrNetworkingTestType,
}

impl MediaErrNetworking {
    fn new(test_type: MediaErrNetworkingTestType) -> Self {
        Self {
            client: reqwest::Client::new(),
            test_type,
        }
    }
}

#[async_trait]
impl RequestExecutor for MediaErrNetworking {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        Err(RequestExecutionError::RequestExecutionFailed {
            status_code: None,
            redirects: None,
            reason: RequestExecutionErrorReason::GenericError {
                error_message: "Execute function is not necessary for these tests".to_string(),
            },
            request_url: request.url().0,
            request_method: request.method(),
        })
    }

    async fn upload(
        &self,
        upload_request: Arc<WpMultipartFormRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        let request = self
            .client
            .request(
                ReqwestRequestExecutor::request_method(upload_request.method()),
                upload_request.url().0.as_str(),
            )
            .headers(upload_request.header_map().to_header_map());
        let mut form = reqwest::multipart::Form::new();

        match self.test_type {
            MediaErrNetworkingTestType::UploadNoData => {
                // don't add the file
            }
        }

        for field in upload_request.form() {
            match field {
                WpMultipartFormField::Text { name, value } => {
                    form = form.text(name, value);
                }
                WpMultipartFormField::File { .. } => {}
            }
        }

        let request = request.multipart(form);
        let mut response =
            request
                .send()
                .await
                .map_err(|e| RequestExecutionError::RequestExecutionFailed {
                    status_code: e.status().map(|s| s.as_u16() as u32),
                    redirects: None,
                    reason: RequestExecutionErrorReason::GenericError {
                        error_message: e.to_string(),
                    },
                    request_url: upload_request.url().0.clone(),
                    request_method: upload_request.method(),
                })?;

        let header_map = std::mem::take(response.headers_mut());
        Ok(WpNetworkResponse {
            status_code: response.status().as_u16() as u32,
            body: response.bytes().await.unwrap().to_vec(),
            response_header_map: Arc::new(WpNetworkHeaderMap::new(header_map)),
            request_url: upload_request.url(),
            request_method: upload_request.method(),
            request_header_map: upload_request.header_map(),
        })
    }

    async fn sleep(&self, millis: u64) {
        tokio::time::sleep(std::time::Duration::from_millis(millis)).await;
    }

    fn cancel(&self, _context: Arc<RequestContext>) {}
}
