use wp_derive::WpDeriveParamsField;

// Minimal mock traits and implementations
trait AppendUrlQueryPairs { fn append_query_pairs(&self, _: &mut QueryPairs); }
trait FromUrlQueryPairs { fn from_url_query_pairs(_: UrlQueryPairsMap) -> Option<Self> where Self: Sized; fn supports_pagination() -> bool; }
struct QueryPairs;
struct UrlQueryPairsMap;
impl QueryPairs {
    fn append_option_query_value_pair<T>(&mut self, _: impl Into<TestListParamsField>, _: Option<&T>) -> &mut Self { self }
    fn append_vec_query_value_pair<T>(&mut self, _: impl Into<TestListParamsField>, _: &[T]) -> &mut Self { self }
}
impl UrlQueryPairsMap {
    fn get<T: Default>(&self, _: impl Into<TestListParamsField>) -> Option<T> { Some(T::default()) }
    fn get_csv<T: Default>(&self, _: impl Into<TestListParamsField>) -> Vec<T> { vec![] }
}

#[derive(WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct TestListParams {
    // Basic field - should serialize to "page"
    pub page: Option<u32>,
    // Snake case field - should serialize to "per_page"
    pub per_page: Option<u32>,
    // Custom field name override - should serialize to "custom_search"
    #[field_name("custom_search")]
    pub search: Option<String>,
    // Field with different naming convention - should serialize to "authorId" (camelCase)
    #[field_name("authorId")]
    pub author_id: Vec<i64>,
    // Multi-word field - should serialize to "search_columns"
    pub search_columns: Vec<String>,
}

fn main() {
    // Verify basic field serialization
    let page: &'static str = TestListParamsField::Page.into();
    assert_eq!("page", page);

    // Verify snake_case field serialization
    let per_page: &'static str = TestListParamsField::PerPage.into();
    assert_eq!("per_page", per_page);

    // Verify custom field_name attribute override
    let search: &'static str = TestListParamsField::Search.into();
    assert_eq!("custom_search", search);

    // Verify field_name attribute with camelCase (different from snake_case default)
    let author_id: &'static str = TestListParamsField::AuthorId.into();
    assert_eq!("authorId", author_id);

    // Verify multi-word field auto snake_case conversion
    let search_columns: &'static str = TestListParamsField::SearchColumns.into();
    assert_eq!("search_columns", search_columns);
}
