use crate::{OptionFromStr, date::WpGmtDateTime, impl_as_query_value_from_to_string};
use std::{borrow::Cow, collections::HashMap, str::FromStr};
use url::{UrlQuery, form_urlencoded};

pub type QueryPairs<'a> = form_urlencoded::Serializer<'a, UrlQuery<'a>>;

pub(crate) trait AppendUrlQueryPairs {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs);
}

pub trait QueryPairsExtension {
    fn append_query_value_pair<'a, T>(&mut self, key: impl Into<&'a str>, value: &T) -> &mut Self
    where
        T: AsQueryValue;
    fn append_option_query_value_pair<'a, T>(
        &mut self,
        key: impl Into<&'a str>,
        value: Option<&T>,
    ) -> &mut Self
    where
        T: AsQueryValue;
    fn append_vec_query_value_pair<'a, T>(
        &mut self,
        key: impl Into<&'a str>,
        value: &[T],
    ) -> &mut Self
    where
        T: AsQueryValue;
}

impl QueryPairsExtension for QueryPairs<'_> {
    fn append_query_value_pair<'a, T>(&mut self, key: impl Into<&'a str>, value: &T) -> &mut Self
    where
        T: AsQueryValue,
    {
        self.append_pair(key.into(), value.as_query_value().as_ref());
        self
    }

    fn append_option_query_value_pair<'a, T>(
        &mut self,
        key: impl Into<&'a str>,
        value: Option<&T>,
    ) -> &mut Self
    where
        T: AsQueryValue,
    {
        if let Some(value) = value {
            self.append_query_value_pair(key, value);
        }
        self
    }

    fn append_vec_query_value_pair<'a, T>(
        &mut self,
        key: impl Into<&'a str>,
        value: &[T],
    ) -> &mut Self
    where
        T: AsQueryValue,
    {
        if !value.is_empty() {
            let mut csv = value.iter().fold(String::new(), |mut acc, s| {
                acc.push_str(s.as_query_value().as_ref());
                acc.push(',');
                acc
            });
            csv.pop(); // remove the last ','
            self.append_pair(key.into(), &csv);
        }
        self
    }
}

pub trait AsQueryValue {
    fn as_query_value(&self) -> impl AsRef<str>;
}

impl_as_query_value_from_to_string!(u32);
impl_as_query_value_from_to_string!(u64);
impl_as_query_value_from_to_string!(i32);
impl_as_query_value_from_to_string!(i64);
impl_as_query_value_from_to_string!(bool);

impl AsQueryValue for &str {
    fn as_query_value(&self) -> impl AsRef<str> {
        self
    }
}

impl AsQueryValue for String {
    fn as_query_value(&self) -> impl AsRef<str> {
        self
    }
}

impl AsQueryValue for WpGmtDateTime {
    fn as_query_value(&self) -> impl AsRef<str> {
        self.to_string()
    }
}

#[derive(Debug)]
pub struct UrlQueryPairsMap<'a> {
    inner: HashMap<Cow<'a, str>, Cow<'a, str>>,
}

impl<'a> UrlQueryPairsMap<'a> {
    pub(crate) fn new(query_pairs: HashMap<Cow<'a, str>, Cow<'a, str>>) -> Self {
        Self { inner: query_pairs }
    }

    pub(crate) fn get<'b, T: FromStr>(&self, key: impl Into<&'b str>) -> Option<T> {
        self.inner.get(key.into()).and_then(|v| v.parse().ok())
    }

    pub(crate) fn get_using_option_from_str<'b, T: OptionFromStr>(
        &self,
        key: impl Into<&'b str>,
    ) -> Option<T> {
        self.inner
            .get(key.into())
            .and_then(|v| T::option_from_str(v).ok().flatten())
    }

    pub(crate) fn get_csv<'b, T: FromStr>(&self, key: impl Into<&'b str>) -> Vec<T> {
        // If the `key` exists as a regular CSV value, parse and return it.
        let key = key.into();
        let result = self.inner.get(key).and_then(|v| {
            v.split(',')
                .map(|s| T::from_str(s).ok())
                .collect::<Option<Vec<_>>>()
        });
        if let Some(values) = result {
            return values;
        }

        // Otherwise, try to get values from PHP-style array parameters: `foo[0]=a&foo[1]=b`.
        self.get_php_array_values(key)
    }

    /// Retrieves values from PHP-style array query parameters.
    ///
    /// WordPress uses PHP's `http_build_query` to generate pagination Link headers.
    /// When an array parameter like `status=["draft", "publish"]` is serialized,
    /// PHP produces `status[0]=draft&status[1]=publish` instead of `status=draft,publish`.
    ///
    /// This function handles that format by scanning all keys matching `key[$int]`,
    /// where `$int` is any valid integer. Non-sequential indices are supported.
    ///
    /// See:
    /// - <https://github.com/WordPress/wordpress-develop/blob/6.9.0/src/wp-includes/functions.php#L1064>
    /// - <https://www.php.net/manual/en/function.http-build-query.php>
    fn get_php_array_values<T: FromStr>(&self, key: &str) -> Vec<T> {
        let mut indexed_values: Vec<(i64, T)> = self
            .inner
            .iter()
            .filter_map(|(k, v)| {
                let k_str = k.as_ref();
                let open_bracket = k_str.find('[')?;
                let close_bracket = k_str.find(']')?;
                if &k_str[..open_bracket] != key || close_bracket != k_str.len() - 1 {
                    return None;
                }
                let index: i64 = k_str[open_bracket + 1..close_bracket].parse().ok()?;
                let parsed = T::from_str(v).ok()?;
                Some((index, parsed))
            })
            .collect();
        indexed_values.sort_by_key(|(index, _)| *index);
        indexed_values.into_iter().map(|(_, v)| v).collect()
    }

    pub(crate) fn get_wp_date_time<'b>(&self, key: impl Into<&'b str>) -> Option<WpGmtDateTime> {
        self.inner
            .get(key.into())
            .and_then(|v| v.parse::<WpGmtDateTime>().ok())
    }
}

pub trait FromUrlQueryPairs
where
    Self: Sized,
{
    fn from_url_query_pairs(query_pairs: UrlQueryPairsMap) -> Option<Self>;

    // Used to avoid unnecessary parsing of the `next` & `prev` headers for params types that don't
    // support pagination.
    //
    // All manually implemented types should return `true` and the implementation for `()` should
    // return `false` since `#[derive(WpDerivedRequest)]` will use `()` for parameter `P` of
    // `ParsedRequest<T, P>`.
    fn supports_pagination() -> bool;
}

impl FromUrlQueryPairs for () {
    fn from_url_query_pairs(_query_pairs: UrlQueryPairsMap) -> Option<Self> {
        None
    }

    fn supports_pagination() -> bool {
        false
    }
}

mod macro_helper {
    #[macro_export]
    macro_rules! impl_as_query_value_from_as_str {
        ($ident: ident) => {
            impl $crate::url_query::AsQueryValue for $ident {
                fn as_query_value(&self) -> impl AsRef<str> {
                    self.as_str()
                }
            }
        };
    }

    #[macro_export]
    macro_rules! impl_as_query_value_from_to_string {
        ($ident: ident) => {
            impl $crate::url_query::AsQueryValue for $ident {
                fn as_query_value(&self) -> impl AsRef<str> {
                    self.to_string()
                }
            }
        };
    }

    #[macro_export]
    macro_rules! impl_as_query_value_for_new_type {
        ($ident: ident) => {
            impl $crate::url_query::AsQueryValue for $ident {
                fn as_query_value(&self) -> impl AsRef<str> {
                    self.0.as_query_value()
                }
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use url::Url;

    #[rstest]
    #[case("foo", 1, "foo=1")]
    #[case("foo", "2", "foo=2")]
    #[case("foo", "2".to_string(), "foo=2")]
    #[case("foo", true, "foo=true")]
    fn test_append_query_value_pair(
        #[case] key: &str,
        #[case] value: impl AsQueryValue,
        #[case] expected_str: &str,
    ) {
        let mut url = Url::parse("https://example.com").unwrap();
        url.query_pairs_mut().append_query_value_pair(key, &value);
        assert_eq!(url.query(), Some(expected_str));
    }

    #[rstest]
    #[case("foo", Some(1), "foo=1")]
    #[case("foo", Some("2"), "foo=2")]
    #[case("foo", Some("2".to_string()), "foo=2")]
    #[case("foo", Some(true), "foo=true")]
    #[case("foo", None::<bool>, "")]
    fn test_append_option_query_value_pair(
        #[case] key: &str,
        #[case] value: Option<impl AsQueryValue>,
        #[case] expected_str: &str,
    ) {
        let mut url = Url::parse("https://example.com").unwrap();
        url.query_pairs_mut()
            .append_option_query_value_pair(key, value.as_ref());
        assert_eq!(url.query(), Some(expected_str));
    }

    #[rstest]
    #[case("foo", vec![1], "foo=1")]
    #[case("foo", vec!["2"], "foo=2")]
    #[case("foo", vec!["1".to_string(), "2".to_string()], "foo=1%2C2")]
    #[case("foo", vec![true, false], "foo=true%2Cfalse")]
    #[case("foo", Vec::<bool>::new(), "")]
    fn test_append_vec_query_value_pair(
        #[case] key: &str,
        #[case] value: Vec<impl AsQueryValue>,
        #[case] expected_str: &str,
    ) {
        let mut url = Url::parse("https://example.com").unwrap();
        url.query_pairs_mut()
            .append_vec_query_value_pair(key, &value);
        assert_eq!(url.query(), Some(expected_str));
    }

    #[test]
    fn test_get_csv_with_regular_csv_value() {
        let mut pairs = HashMap::new();
        pairs.insert(Cow::Borrowed("status"), Cow::Borrowed("draft,publish"));
        pairs.insert(Cow::Borrowed("page"), Cow::Borrowed("1"));

        let map = UrlQueryPairsMap::new(pairs);
        let statuses: Vec<String> = map.get_csv("status");
        assert_eq!(statuses, vec!["draft".to_string(), "publish".to_string()]);
    }

    #[test]
    fn test_get_csv_with_php_array_single_value() {
        let mut pairs = HashMap::new();
        pairs.insert(Cow::Borrowed("status[0]"), Cow::Borrowed("any"));
        pairs.insert(Cow::Borrowed("page"), Cow::Borrowed("2"));

        let map = UrlQueryPairsMap::new(pairs);
        let statuses: Vec<String> = map.get_csv("status");
        assert_eq!(statuses, vec!["any".to_string()]);
        assert_eq!(map.get::<u32>("page"), Some(2));
    }

    #[test]
    fn test_get_csv_with_php_array_multiple_values() {
        let mut pairs = HashMap::new();
        pairs.insert(Cow::Borrowed("status[0]"), Cow::Borrowed("draft"));
        pairs.insert(Cow::Borrowed("status[1]"), Cow::Borrowed("publish"));
        pairs.insert(Cow::Borrowed("page"), Cow::Borrowed("1"));

        let map = UrlQueryPairsMap::new(pairs);
        let statuses: Vec<String> = map.get_csv("status");
        assert_eq!(statuses, vec!["draft".to_string(), "publish".to_string()]);
    }

    #[test]
    fn test_get_csv_with_php_array_with_gaps() {
        let mut pairs = HashMap::new();
        pairs.insert(Cow::Borrowed("status[0]"), Cow::Borrowed("draft"));
        pairs.insert(Cow::Borrowed("status[2]"), Cow::Borrowed("publish"));
        pairs.insert(Cow::Borrowed("status[4]"), Cow::Borrowed("trash"));

        let map = UrlQueryPairsMap::new(pairs);
        let statuses: Vec<String> = map.get_csv("status");
        assert_eq!(
            statuses,
            vec![
                "draft".to_string(),
                "publish".to_string(),
                "trash".to_string()
            ]
        );
    }

    #[test]
    fn test_get_csv_prefers_regular_key_over_php_array() {
        let mut pairs = HashMap::new();
        pairs.insert(Cow::Borrowed("status"), Cow::Borrowed("private"));
        pairs.insert(Cow::Borrowed("status[0]"), Cow::Borrowed("draft"));

        let map = UrlQueryPairsMap::new(pairs);
        let statuses: Vec<String> = map.get_csv("status");
        assert_eq!(statuses, vec!["private".to_string()]);
    }

    #[test]
    fn test_get_csv_with_php_array_from_url() {
        let url =
            Url::parse("http://localhost/wp-json/wp/v2/posts?status%5B0%5D=any&per_page=60&page=2")
                .unwrap();
        let pairs: HashMap<_, _> = url.query_pairs().into_iter().collect();
        let map = UrlQueryPairsMap::new(pairs);

        let statuses: Vec<String> = map.get_csv("status");
        assert_eq!(statuses, vec!["any".to_string()]);
        assert_eq!(map.get::<u32>("page"), Some(2));
        assert_eq!(map.get::<u32>("per_page"), Some(60));
    }
}
