use wp_api::{
    auth::WpAuthenticationProvider,
    media::{MediaCreateParams, MediaId, MediaListParams, MediaUpdateParams},
    posts::WpApiParamPostsOrderBy,
    prelude::*,
    request::endpoint::media_endpoint::MediaUploadRequest,
    users::UserId,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn create_media_err_cannot_create() {
    api_client_as_subscriber()
        .media()
        .create(
            MediaCreateParams::default(),
            MEDIA_TEST_FILE_PATH.to_string(),
            MEDIA_TEST_FILE_CONTENT_TYPE.to_string(),
            None,
        )
        .await
        .assert_wp_error(WpErrorCode::CannotCreate)
}

#[tokio::test]
#[parallel]
async fn create_media_err_upload_no_data() {
    api_client_with_medir_err_networking(MediaErrNetworkingTestType::UploadNoData)
        .media()
        .create(
            MediaCreateParams::default(),
            MEDIA_TEST_FILE_PATH.to_string(),
            MEDIA_TEST_FILE_CONTENT_TYPE.to_string(),
            None,
        )
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
        _request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        Err(RequestExecutionError::RequestExecutionFailed {
            status_code: None,
            redirects: None,
            reason: RequestExecutionErrorReason::GenericError {
                error_message: "Execute function is not necessary for these tests".to_string(),
            },
        })
    }

    async fn upload_media(
        &self,
        media_upload_request: Arc<MediaUploadRequest>,
    ) -> Result<WpNetworkResponse, MediaUploadRequestExecutionError> {
        let mut request = self
            .client
            .request(
                ReqwestRequestExecutor::request_method(media_upload_request.method()),
                media_upload_request.url().0.as_str(),
            )
            .headers(media_upload_request.header_map().to_header_map());
        let mut file_header_map = HeaderMap::new();
        file_header_map.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_str(&media_upload_request.file_content_type()).unwrap(),
        );
        let mut form = reqwest::multipart::Form::new();
        match self.test_type {
            MediaErrNetworkingTestType::UploadNoData => {
                // don't add the file
            }
        }
        for (k, v) in media_upload_request.media_params() {
            form = form.text(k, v)
        }
        request = request.multipart(form);

        let mut response = request.send().await.map_err(|err| {
            MediaUploadRequestExecutionError::RequestExecutionFailed {
                status_code: err.status().map(|s| s.as_u16()),
                redirects: None,
                reason: RequestExecutionErrorReason::GenericError {
                    error_message: err.to_string(),
                },
            }
        })?;

        let header_map = std::mem::take(response.headers_mut());
        Ok(WpNetworkResponse {
            status_code: response.status().as_u16(),
            body: response.bytes().await.unwrap().to_vec(),
            response_header_map: Arc::new(WpNetworkHeaderMap::new(header_map)),
            request_url: media_upload_request.url(),
            request_header_map: media_upload_request.header_map(),
        })
    }

    async fn sleep(&self, millis: u64) {
        tokio::time::sleep(std::time::Duration::from_millis(millis)).await;
    }
}
