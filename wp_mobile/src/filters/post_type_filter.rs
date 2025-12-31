/// Filter for querying post types in a collection
///
/// Represents domain-level filtering criteria for post types.
/// Since there are typically few post types per site, filtering happens
/// at the collection/database level rather than at the API level.
///
/// # Default Behavior
/// By default, only viewable post types are returned (`viewable = Some(true)`),
/// as these are the types typically shown to users in a UI.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PostTypeFilter {
    /// Filter by viewable status
    ///
    /// - `None`: No filtering - returns all post types regardless of viewable status
    /// - `Some(true)`: Returns only post types where viewable is explicitly true
    /// - `Some(false)`: Returns only post types where viewable is explicitly false
    ///
    /// Note: Post types with `viewable == None` are only included when this filter is `None`.
    pub viewable: Option<bool>,
}

impl Default for PostTypeFilter {
    fn default() -> Self {
        Self {
            viewable: Some(true),
        }
    }
}
