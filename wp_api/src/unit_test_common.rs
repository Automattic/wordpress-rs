use crate::url_query::AppendUrlQueryPairs;
use url::Url;

#[cfg(test)]
pub fn assert_expected_query_pairs(params: impl AppendUrlQueryPairs, expected_query: &str) {
    let mut url = Url::parse("https://example.com").unwrap();
    params.append_query_pairs(&mut url.query_pairs_mut());
    assert_eq!(url.query().unwrap(), expected_query);
}
