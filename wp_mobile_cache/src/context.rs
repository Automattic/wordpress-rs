use crate::{SqliteDbError, term_relationships::DbTermRelationship};
use rusqlite::Row;

/// Base trait for WordPress REST API context types.
///
/// This trait is entity-agnostic and only handles table naming and context identification.
/// Entity-specific traits (PostContext, CommentContext, etc.) extend this.
pub trait IsContext: 'static + Copy {
    /// The context suffix used in table names.
    ///
    /// # Example
    /// ```
    /// // EditContext::context_suffix() => "edit"
    /// // ViewContext::context_suffix() => "view"
    /// // EmbedContext::context_suffix() => "embed"
    /// ```
    fn context_suffix() -> &'static str;

    /// The WpContext enum variant for this context.
    fn wp_context() -> wp_api::WpContext;

    /// Generate the full table name for a given entity type prefix.
    ///
    /// # Example
    /// ```
    /// // EditContext::table_name("posts") => "posts_edit_context"
    /// // EditContext::table_name("comments") => "comments_edit_context"
    /// ```
    fn table_name(prefix: &str) -> String {
        format!("{}_{}_context", prefix, Self::context_suffix())
    }
}

/// Entity-specific context trait for Posts.
///
/// Associates a context with post-specific types and provides database row mapping.
pub trait PostContext: IsContext {
    /// The context-specific post entity type (e.g., AnyPostWithEditContext)
    type Post;

    /// The context-specific database wrapper type (e.g., DbAnyPostWithEditContext)
    type DbPost;

    /// Construct DbPost from a database row with associated term relationships.
    ///
    /// This method is implemented in the repository module where database logic belongs.
    fn from_row_with_terms(
        row: &Row,
        term_relationships: Vec<DbTermRelationship>,
    ) -> Result<Self::DbPost, SqliteDbError>;
}

// Future entity-specific traits would go here:
// pub trait CommentContext: IsContext { ... }
// pub trait UserContext: IsContext { ... }

/// Marker type for Edit context
#[derive(Debug, Clone, Copy)]
pub struct EditContext;

/// Marker type for View context
#[derive(Debug, Clone, Copy)]
pub struct ViewContext;

/// Marker type for Embed context
#[derive(Debug, Clone, Copy)]
pub struct EmbedContext;

// Implement base IsContext trait for each marker type
impl IsContext for EditContext {
    fn context_suffix() -> &'static str {
        "edit"
    }

    fn wp_context() -> wp_api::WpContext {
        wp_api::WpContext::Edit
    }
}

impl IsContext for ViewContext {
    fn context_suffix() -> &'static str {
        "view"
    }

    fn wp_context() -> wp_api::WpContext {
        wp_api::WpContext::View
    }
}

impl IsContext for EmbedContext {
    fn context_suffix() -> &'static str {
        "embed"
    }

    fn wp_context() -> wp_api::WpContext {
        wp_api::WpContext::Embed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_name_generation() {
        assert_eq!(EditContext::table_name("posts"), "posts_edit_context");
        assert_eq!(ViewContext::table_name("posts"), "posts_view_context");
        assert_eq!(EmbedContext::table_name("posts"), "posts_embed_context");

        assert_eq!(EditContext::table_name("comments"), "comments_edit_context");
        assert_eq!(ViewContext::table_name("comments"), "comments_view_context");
    }

    #[test]
    fn test_context_suffix() {
        assert_eq!(EditContext::context_suffix(), "edit");
        assert_eq!(ViewContext::context_suffix(), "view");
        assert_eq!(EmbedContext::context_suffix(), "embed");
    }

    #[test]
    fn test_wp_context() {
        assert_eq!(EditContext::wp_context(), wp_api::WpContext::Edit);
        assert_eq!(ViewContext::wp_context(), wp_api::WpContext::View);
        assert_eq!(EmbedContext::wp_context(), wp_api::WpContext::Embed);
    }
}
