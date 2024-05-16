#[cfg(test)]
pub fn assert_expected_query_pairs<'a>(
    query_pairs: impl IntoIterator<Item = (&'a str, String)>,
    expected_pairs: &[(&'a str, &str)],
) {
    let mut query_pairs = query_pairs.into_iter().collect::<Vec<_>>();
    let mut expected_pairs: Vec<(&str, String)> = expected_pairs
        .iter()
        .map(|(k, v)| (*k, v.to_string()))
        .collect();
    // The order of query pairs doesn't matter
    query_pairs.sort();
    expected_pairs.sort();
    assert_eq!(query_pairs, expected_pairs);
}
