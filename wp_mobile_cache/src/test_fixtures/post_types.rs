use std::collections::HashMap;
use wp_api::{
    JsonValue,
    post_types::{
        PostTypeCapabilities, PostTypeDetailsWithEditContext, PostTypeLabels,
        PostTypeSupports, PostTypeSupportsMap, PostTypeVisibility,
    },
};

/// Builder for creating test post types with configurable attributes.
///
/// Simplifies creation of post type test data with different viewable, show_ui,
/// and hierarchical values for testing filtering logic.
///
/// # Example
///
/// ```rust,ignore
/// // Create a viewable, UI-visible, hierarchical post type (like 'page')
/// let page = PostTypeBuilder::new("page")
///     .viewable(true)
///     .show_ui(true)
///     .hierarchical(true)
///     .build();
///
/// // Create a non-viewable, hidden post type (like 'revision')
/// let revision = PostTypeBuilder::new("revision")
///     .viewable(false)
///     .show_ui(false)
///     .hierarchical(false)
///     .build();
/// ```
pub struct PostTypeBuilder {
    post_type: PostTypeDetailsWithEditContext,
}

impl PostTypeBuilder {
    /// Create a new builder with defaults matching a typical viewable post type.
    ///
    /// Default values:
    /// - viewable: true
    /// - show_ui: true
    /// - hierarchical: false
    /// - rest_base: same as slug
    pub fn new(slug: &str) -> Self {
        let post_type = PostTypeDetailsWithEditContext {
            capabilities: HashMap::from([
                (PostTypeCapabilities::EditPost, "edit_posts".to_string()),
                (PostTypeCapabilities::ReadPost, "read".to_string()),
                (PostTypeCapabilities::DeletePost, "delete_posts".to_string()),
            ]),
            description: format!("Test {} post type", slug),
            hierarchical: false,
            viewable: true,
            labels: PostTypeLabels {
                name: slug.to_string(),
                singular_name: slug.to_string(),
                add_new: "Add New".to_string(),
                add_new_item: format!("Add New {}", slug),
                edit_item: format!("Edit {}", slug),
                new_item: format!("New {}", slug),
                view_item: format!("View {}", slug),
                view_items: format!("View {}", slug),
                search_items: format!("Search {}", slug),
                not_found: format!("No {} found", slug),
                not_found_in_trash: format!("No {} found in Trash", slug),
                parent_item_colon: None,
                all_items: format!("All {}", slug),
                archives: format!("{} Archives", slug),
                attributes: format!("{} Attributes", slug),
                insert_into_item: format!("Insert into {}", slug),
                uploaded_to_this_item: format!("Uploaded to this {}", slug),
                featured_image: "Featured Image".to_string(),
                set_featured_image: "Set featured image".to_string(),
                remove_featured_image: "Remove featured image".to_string(),
                use_featured_image: "Use as featured image".to_string(),
                filter_items_list: format!("Filter {} list", slug),
                filter_by_date: "Filter by date".to_string(),
                items_list_navigation: format!("{} list navigation", slug),
                items_list: format!("{} list", slug),
                item_published: format!("{} published", slug),
                item_published_privately: format!("{} published privately", slug),
                item_reverted_to_draft: format!("{} reverted to draft", slug),
                item_trashed: format!("{} trashed", slug),
                item_scheduled: format!("{} scheduled", slug),
                item_updated: format!("{} updated", slug),
                item_link: format!("{} Link", slug),
                item_link_description: format!("A link to a {}", slug),
                menu_name: slug.to_string(),
                name_admin_bar: slug.to_string(),
            },
            name: slug.to_string(),
            slug: slug.to_string(),
            supports: PostTypeSupportsMap {
                map: HashMap::from([
                    (PostTypeSupports::Title, JsonValue::Bool(true)),
                    (PostTypeSupports::Editor, JsonValue::Bool(true)),
                ]),
            },
            has_archive: false,
            taxonomies: vec![],
            rest_base: slug.to_string(),
            rest_namespace: "wp/v2".to_string(),
            visibility: PostTypeVisibility {
                show_in_nav_menus: true,
                show_ui: true,
            },
            icon: None,
        };

        Self { post_type }
    }

    /// Set the viewable attribute.
    pub fn viewable(mut self, viewable: bool) -> Self {
        self.post_type.viewable = viewable;
        self
    }

    /// Set the show_ui attribute (via visibility.show_ui).
    pub fn show_ui(mut self, show_ui: bool) -> Self {
        self.post_type.visibility.show_ui = show_ui;
        self
    }

    /// Set the hierarchical attribute.
    pub fn hierarchical(mut self, hierarchical: bool) -> Self {
        self.post_type.hierarchical = hierarchical;
        self
    }

    /// Set the rest_base (defaults to slug if not set).
    pub fn rest_base(mut self, rest_base: &str) -> Self {
        self.post_type.rest_base = rest_base.to_string();
        self
    }

    /// Build the final PostTypeDetailsWithEditContext.
    pub fn build(self) -> PostTypeDetailsWithEditContext {
        self.post_type
    }
}
