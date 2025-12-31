/// Filter for querying post types in a collection
///
/// Represents domain-level filtering criteria for post types.
/// Since there are typically few post types per site, filtering happens
/// at the collection/database level rather than at the API level.
///
/// # Default Behavior
/// By default, only viewable and UI-visible post types are returned
/// (`viewable = Some(true)`, `show_ui = Some(true)`), as these are the types
/// typically shown to users in a UI. The `hierarchical` filter defaults to `None`
/// (no filtering).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PostTypeFilter {
    /// Filter by viewable status
    ///
    /// - `None`: No filtering - returns all post types regardless of viewable status
    /// - `Some(true)`: Returns only post types where viewable is true
    /// - `Some(false)`: Returns only post types where viewable is false
    pub viewable: Option<bool>,

    /// Filter by show_ui status (from visibility.show_ui)
    ///
    /// - `None`: No filtering - returns all post types regardless of show_ui status
    /// - `Some(true)`: Returns only post types where visibility.show_ui is true
    /// - `Some(false)`: Returns only post types where visibility.show_ui is false
    pub show_ui: Option<bool>,

    /// Filter by hierarchical support
    ///
    /// - `None`: No filtering - returns all post types regardless of hierarchical status
    /// - `Some(true)`: Returns only hierarchical post types (e.g., pages with parent/child)
    /// - `Some(false)`: Returns only flat post types (e.g., posts without hierarchy)
    #[uniffi(default = None)]
    pub hierarchical: Option<bool>,
}

impl Default for PostTypeFilter {
    fn default() -> Self {
        Self {
            viewable: Some(true),
            show_ui: Some(true),
            hierarchical: None,
        }
    }
}
