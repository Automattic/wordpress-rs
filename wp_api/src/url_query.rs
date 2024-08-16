use url::{form_urlencoded, UrlQuery};

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

impl AsQueryValue for u32 {
    fn as_query_value(&self) -> impl AsRef<str> {
        self.to_string()
    }
}

impl AsQueryValue for i32 {
    fn as_query_value(&self) -> impl AsRef<str> {
        self.to_string()
    }
}

impl AsQueryValue for bool {
    fn as_query_value(&self) -> impl AsRef<str> {
        self.to_string()
    }
}

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
