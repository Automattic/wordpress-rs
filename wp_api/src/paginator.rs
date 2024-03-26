use std::sync::*;

use super::*;

#[uniffi::export(with_foreign)]
pub trait BlockingAPIClient: Send + Sync {
    fn send_request(
        &self,
        request: WPNetworkRequest,
    ) -> Result<WPNetworkResponse, BlockingAPIClientError>;
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum BlockingAPIClientError {
    #[error("Native clients couldn't receive a HTTP response.")]
    NativeClientError { data: Vec<u8> },
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum PaginationError {
    #[error("Reached the end of the list")]
    ReachedEnd,

    #[error("A REST API error occurred")]
    APIError { error: WPApiError },

    #[error("Native clients couldn't receive a HTTP response.")]
    NativeClientError { error: BlockingAPIClientError },

    #[error("A unknown error occurred")]
    Unknown,
}

#[derive(uniffi::Object)]
pub struct Paginator {
    client: Arc<dyn BlockingAPIClient>,
    api_helper: Arc<WPApiHelper>,
    route: String,
    query: Vec<QueryItem>,
    per_page: u32,
    state: RwLock<PaginationState>,
}

#[derive(Debug, Default)]
struct PaginationState {
    current_page: u32,
    total: Option<u32>,
    total_pages: Option<u32>,
}

#[uniffi::export]
impl Paginator {
    #[uniffi::constructor]
    pub fn new(
        client: Arc<dyn BlockingAPIClient>,
        api_helper: Arc<WPApiHelper>,
        route: String,
        query: Option<Vec<QueryItem>>,
        per_page: Option<u32>,
    ) -> Self {
        Self {
            client,
            api_helper,
            per_page: per_page.unwrap_or(100),
            state: Default::default(),
            route,
            query: query.unwrap_or_default(),
        }
    }

    fn next_page(&self) -> Result<PostListResponse, PaginationError> {
        let mut state = self.state.write().map_err(|_| PaginationError::Unknown)?;

        let pagination_params = [
            QueryItem { name: "page".to_string(), value: (state.current_page + 1).to_string() },
            QueryItem { name: "per_page".to_string(), value: self.per_page.to_string() },
        ];
        let query = self.query.iter().chain(pagination_params.iter());
        let request = self.api_helper.request(&self.route, query, Some(RequestMethod::GET)).map_err(|err| PaginationError::APIError { error: err })?;

        let response = self
            .client
            .send_request(request)
            .map_err(|err| PaginationError::NativeClientError { error: err })?;

        let response = parse_post_list_response(response).map_err(|err| {
            match err {
                WPApiError::ClientError {
                    error_type,
                    status_code,
                } => {
                    // TODO: check REST API error code "invalid_post_page_number"
                    if status_code == 400 {
                        PaginationError::ReachedEnd
                    } else {
                        PaginationError::APIError {
                            error: WPApiError::ClientError {
                                error_type,
                                status_code,
                            },
                        }
                    }
                }
                _ => PaginationError::APIError { error: err },
            }
        })?;

        state.current_page += 1;
        state.total = response.total;
        state.total_pages = response.total_pages;

        // TODO: filter duplicated result
        Ok(response)
    }
}
