use std::sync::Arc;

use url::{form_urlencoded, UrlQuery};

use crate::{date::WpDateTimeISO8601, impl_as_query_value_from_to_string};

pub(crate) type QueryPairs<'a> = form_urlencoded::Serializer<'a, UrlQuery<'a>>;

pub(crate) trait AppendUrlQueryPairs {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs);
}

pub(crate) trait QueryPairsExtension {
    fn append_query_value_pair<T>(&mut self, key: &str, value: &T) -> &mut Self
    where
        T: AsQueryValue;
    fn append_option_query_value_pair<T>(&mut self, key: &str, value: Option<&T>) -> &mut Self
    where
        T: AsQueryValue;
    fn append_vec_query_value_pair<T>(&mut self, key: &str, value: &[T]) -> &mut Self
    where
        T: AsQueryValue;
}

impl QueryPairsExtension for QueryPairs<'_> {
    fn append_query_value_pair<T>(&mut self, key: &str, value: &T) -> &mut Self
    where
        T: AsQueryValue,
    {
        self.append_pair(key, value.as_query_value().as_ref());
        self
    }

    fn append_option_query_value_pair<T>(&mut self, key: &str, value: Option<&T>) -> &mut Self
    where
        T: AsQueryValue,
    {
        if let Some(value) = value {
            self.append_query_value_pair(key, value);
        }
        self
    }

    fn append_vec_query_value_pair<T>(&mut self, key: &str, value: &[T]) -> &mut Self
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
            self.append_pair(key, &csv);
        }
        self
    }
}

pub(crate) trait AsQueryValue {
    fn as_query_value(&self) -> impl AsRef<str>;
}

impl_as_query_value_from_to_string!(u32);
impl_as_query_value_from_to_string!(i32);
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

impl AsQueryValue for Arc<WpDateTimeISO8601> {
    fn as_query_value(&self) -> impl AsRef<str> {
        self.to_rfc3339()
    }
}

mod macro_helper {
    #[macro_export]
    macro_rules! impl_as_query_value_from_as_str {
        ($ident: ident) => {
            impl AsQueryValue for $ident {
                fn as_query_value(&self) -> impl AsRef<str> {
                    self.as_str()
                }
            }
        };
    }

    #[macro_export]
    macro_rules! impl_as_query_value_from_to_string {
        ($ident: ident) => {
            impl AsQueryValue for $ident {
                fn as_query_value(&self) -> impl AsRef<str> {
                    self.to_string()
                }
            }
        };
    }

    #[macro_export]
    macro_rules! impl_as_query_value_for_new_type {
        ($ident: ident) => {
            impl AsQueryValue for $ident {
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
}
