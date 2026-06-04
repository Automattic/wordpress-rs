use crate::{
    SparseField,
    date::WpGmtDateTime,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct SparseBlockDirectoryItem {
    pub name: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub id: Option<String>,
    pub rating: Option<f64>,
    pub rating_count: Option<i64>,
    pub active_installs: Option<i64>,
    pub author_block_rating: Option<f64>,
    pub author_block_count: Option<i64>,
    pub author: Option<String>,
    pub icon: Option<String>,
    pub last_updated: Option<WpGmtDateTime>,
    pub humanized_updated: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct BlockDirectoryItem {
    pub name: String,
    pub title: String,
    pub description: String,
    pub id: String,
    pub rating: f64,
    pub rating_count: i64,
    pub active_installs: i64,
    pub author_block_rating: f64,
    pub author_block_count: i64,
    pub author: String,
    pub icon: String,
    pub last_updated: WpGmtDateTime,
    pub humanized_updated: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SparseBlockDirectoryItemField {
    Name,
    Title,
    Description,
    Id,
    Rating,
    RatingCount,
    ActiveInstalls,
    AuthorBlockRating,
    AuthorBlockCount,
    Author,
    Icon,
    LastUpdated,
    HumanizedUpdated,
}

impl SparseField for SparseBlockDirectoryItemField {
    fn as_mapped_field_name(&self) -> &str {
        match self {
            Self::Name => "name",
            Self::Title => "title",
            Self::Description => "description",
            Self::Id => "id",
            Self::Rating => "rating",
            Self::RatingCount => "rating_count",
            Self::ActiveInstalls => "active_installs",
            Self::AuthorBlockRating => "author_block_rating",
            Self::AuthorBlockCount => "author_block_count",
            Self::Author => "author",
            Self::Icon => "icon",
            Self::LastUpdated => "last_updated",
            Self::HumanizedUpdated => "humanized_updated",
        }
    }
}

#[derive(Debug, uniffi::Record)]
pub struct BlockDirectorySearchParams {
    /// Limit result set to blocks matching the search term.
    pub term: String,
    /// Current page of the collection. Default: 1
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// Maximum number of items to be returned in result set. Default: 10
    #[uniffi(default = None)]
    pub per_page: Option<u32>,
}

impl BlockDirectorySearchParams {
    pub fn new(term: String) -> Self {
        Self {
            term,
            page: None,
            per_page: None,
        }
    }
}

impl AppendUrlQueryPairs for &BlockDirectorySearchParams {
    fn append_query_pairs(&self, query_pairs: &mut QueryPairs) {
        query_pairs.append_query_value_pair("term", &self.term);
        query_pairs.append_option_query_value_pair("page", self.page.as_ref());
        query_pairs.append_option_query_value_pair("per_page", self.per_page.as_ref());
    }
}
