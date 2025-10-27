mod edit;
mod embed;
mod view;

pub use edit::DbAnyPostWithEditContext;
pub use embed::DbAnyPostWithEmbedContext;
pub use view::DbAnyPostWithViewContext;

use crate::term_relationships::DbTermRelationship;
use wp_api::{taxonomies::TaxonomyType, terms::TermId};

/// Extract categories and tags from term relationships.
///
/// This is a shared helper used by all post context mappings.
/// Domain-specific logic for separating categories from tags is handled here.
///
/// # Arguments
/// * `term_relationships` - Term relationships loaded from term_relationships table
///
/// # Returns
/// Tuple of (categories, tags) as vectors of TermId
pub(super) fn extract_categories_and_tags(
    term_relationships: Vec<DbTermRelationship>,
) -> (Vec<TermId>, Vec<TermId>) {
    term_relationships.into_iter().fold(
        (Vec::new(), Vec::new()),
        |(mut cats, mut tags), relationship| {
            match relationship.taxonomy_type {
                TaxonomyType::Category => cats.push(relationship.term_id),
                TaxonomyType::PostTag => tags.push(relationship.term_id),
                _ => {} // Ignore other taxonomy types for posts
            }
            (cats, tags)
        },
    )
}
