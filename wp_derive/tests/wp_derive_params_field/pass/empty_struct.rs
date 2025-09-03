use wp_derive::WpDeriveParamsField;

// Minimal mock traits
trait AppendUrlQueryPairs { fn append_query_pairs(&self, _: &mut QueryPairs); }
trait FromUrlQueryPairs { fn from_url_query_pairs(_: UrlQueryPairsMap) -> Option<Self> where Self: Sized; fn supports_pagination() -> bool; }
struct QueryPairs;
struct UrlQueryPairsMap;

#[derive(WpDeriveParamsField)]
#[supports_pagination(false)]
pub struct EmptyParams {}

fn main() {}