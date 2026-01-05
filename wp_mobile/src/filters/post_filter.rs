use wp_api::posts::{PostListParams, PostStatus};

/// Filter for querying posts in a collection
///
/// Represents domain-level filtering criteria for posts. This is separate
/// from API parameters to allow future cache-only filters or domain-specific
/// abstractions.
#[derive(Debug, Clone, Default, PartialEq, Eq, uniffi::Record)]
pub struct AnyPostFilter {
    /// Filter by post status (publish, draft, etc.)
    #[uniffi(default = None)]
    pub status: Option<PostStatus>,
}

impl AnyPostFilter {
    /// Convert filter to API parameters for network requests
    ///
    /// This is a helper method (not a `From` trait impl) to signal that
    /// this is a one-way transformation from domain model to API wire format.
    pub fn to_list_params(&self) -> PostListParams {
        let mut params = PostListParams::default();

        if let Some(status) = &self.status {
            params.status = vec![status.clone()];
        }

        params
    }
}
