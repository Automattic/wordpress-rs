use crate::request::FromUrlQueryPairs;
use crate::url_query::AppendUrlQueryPairs;
use crate::UrlQueryPairsMap;
use url::Url;

#[cfg(test)]
pub fn assert_expected_query_pairs(params: impl AppendUrlQueryPairs, expected_query: &str) {
    let mut url = Url::parse("https://example.com").unwrap();
    params.append_query_pairs(&mut url.query_pairs_mut());
    assert_eq!(url.query(), Some(expected_query));
}

#[cfg(test)]
pub fn assert_expected_and_from_query_pairs<P>(params: P, expected_query: &str)
where
    P: AppendUrlQueryPairs + FromUrlQueryPairs + std::fmt::Debug + PartialEq,
{
    let mut url = Url::parse("https://example.com").unwrap();
    params.append_query_pairs(&mut url.query_pairs_mut());
    assert_eq!(url.query(), Some(expected_query));

    let parsed_params = P::from_url_query_pairs(UrlQueryPairsMap::new(
        url.query_pairs().into_iter().collect(),
    ));
    assert_eq!(Some(params), parsed_params);
}
