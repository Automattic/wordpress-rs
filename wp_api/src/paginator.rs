use std::sync::*;

use super::*;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum PaginationError {
    #[error("Reached the end of the list")]
    ReachedEnd,

    #[error("A REST API error occurred")]
    APIError { error: WPApiError },

    #[error("A unknown error occurred")]
    Unknown,
}

#[derive(uniffi::Object)]
pub struct Paginator {
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
        api_helper: Arc<WPApiHelper>,
        route: String,
        query: Option<Vec<QueryItem>>,
        per_page: Option<u32>,
    ) -> Self {
        Self {
            api_helper,
            per_page: per_page.unwrap_or(100),
            state: Default::default(),
            route,
            query: query.unwrap_or_default(),
        }
    }

    fn next_page(&self) -> Result<WPNetworkRequest, PaginationError> {
        let state = self.state.read().map_err(|_| PaginationError::Unknown)?;

        let pagination_params = [
            QueryItem {
                name: "page".to_string(),
                value: (state.current_page + 1).to_string(),
            },
            QueryItem {
                name: "per_page".to_string(),
                value: self.per_page.to_string(),
            },
        ];
        let query = self.query.iter().chain(pagination_params.iter());
        self.api_helper
            .request(&self.route, query, Some(RequestMethod::GET))
            .map_err(|err| PaginationError::APIError { error: err })
    }

    fn receive(&self, response: WPNetworkResponse) -> Result<Vec<u8>, PaginationError> {
        let mut state = self.state.write().map_err(|_| PaginationError::Unknown)?;
        let response = parse_pagination_response(response).map_err(|err| {
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
        Ok(response.json)
    }
}
