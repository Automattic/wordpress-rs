use crate::{
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    wp_com::language::WPComLanguage,
};
use serde::{Deserialize, Serialize};

/// Parameters for the stats tags endpoint.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct StatsTagsParams {
    /// The maximum number of tags to return.
    #[uniffi(default = None)]
    pub max: Option<u32>,
    /// The locale for the response.
    #[uniffi(default = None)]
    pub locale: Option<WPComLanguage>,
}

impl AppendUrlQueryPairs for StatsTagsParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("max", self.max.as_ref())
            .append_option_query_value_pair("locale", self.locale.as_ref());
    }
}

/// Response from the stats tags endpoint.
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsTagsResponse {
    /// The date for the stats query.
    pub date: String,
    /// The list of tag groups with their view counts.
    pub tags: Vec<StatsTagsGroup>,
}

/// A group of tags with a combined view count.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsTagsGroup {
    /// The tags in this group.
    pub tags: Vec<StatsTagEntry>,
    /// The number of views for this tag group.
    pub views: u64,
}

/// A single tag or category entry.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsTagEntry {
    /// The type of the entry (e.g., "tag" or "category").
    #[serde(rename = "type")]
    pub tag_type: String,
    /// The name of the tag or category.
    pub name: String,
    /// The link to the tag or category page.
    pub link: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_tags_params_serialization() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/tags")
                .expect("Failed to parse url");

        let params = StatsTagsParams {
            max: Some(7),
            locale: Some(WPComLanguage::English),
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/tags?max=7&locale=en"
        );
    }

    #[test]
    fn test_stats_tags_params_serialization_empty() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/tags")
                .expect("Failed to parse url");

        let params = StatsTagsParams::default();

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/tags?"
        );
    }

    #[test]
    fn test_stats_tags_response_deserialization() {
        let json_file_path = "tests/wpcom/stats_tags/tags-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTagsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-03-12");
        assert_eq!(response.tags.len(), 3);

        let first = &response.tags[0];
        assert_eq!(first.views, 98);
        assert_eq!(first.tags.len(), 1);
        assert_eq!(first.tags[0].tag_type, "category");
        assert_eq!(first.tags[0].name, "Uncategorized");

        let second = &response.tags[1];
        assert_eq!(second.views, 15);
        assert_eq!(second.tags[0].tag_type, "tag");
        assert_eq!(second.tags[0].name, "snaps");
    }

    #[test]
    fn test_stats_tags_response_deserialization_empty() {
        let json_file_path = "tests/wpcom/stats_tags/tags-02-empty.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTagsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-03-12");
        assert!(response.tags.is_empty());
    }
}
