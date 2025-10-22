//! Multi-site isolation tests for TermRelationshipRepository.
//!
//! These tests verify that term relationships are correctly isolated between sites.

use super::{posts::PostRepository, term_relationships::TermRelationshipRepository};
use crate::{
    DbSite, RowId,
    test_fixtures::posts::PostBuilder,
    test_helpers::{create_test_site, test_db},
};
use rstest::*;
use rusqlite::Connection;
use wp_api::{taxonomies::TaxonomyType, terms::TermId};

#[rstest]
fn test_term_relationships_isolated_by_site(mut test_db: Connection) {
    let post_repo = PostRepository;
    let term_repo = TermRelationshipRepository;
    let site1 = DbSite { row_id: RowId(1) };
    let site2 = create_test_site(&test_db, 2);

    // Insert post in site 1 with categories
    let post1 = PostBuilder::new()
        .with_categories(vec![TermId(1), TermId(2)])
        .build();
    let rowid1 = post_repo.upsert(&mut test_db, &site1, &post1).unwrap();

    // Insert post in site 2 with same categories
    let post2 = PostBuilder::new()
        .with_categories(vec![TermId(1), TermId(2)])
        .build();
    let rowid2 = post_repo.upsert(&mut test_db, &site2, &post2).unwrap();

    // Verify site 1's terms
    let site1_terms = term_repo
        .get_all_terms_for_object(&test_db, &site1, rowid1)
        .unwrap();
    let site1_categories = site1_terms.get(&TaxonomyType::Category).unwrap();
    assert_eq!(site1_categories.len(), 2);
    assert!(site1_categories.contains(&TermId(1)));
    assert!(site1_categories.contains(&TermId(2)));

    // Verify site 2's terms
    let site2_terms = term_repo
        .get_all_terms_for_object(&test_db, &site2, rowid2)
        .unwrap();
    let site2_categories = site2_terms.get(&TaxonomyType::Category).unwrap();
    assert_eq!(site2_categories.len(), 2);
    assert!(site2_categories.contains(&TermId(1)));
    assert!(site2_categories.contains(&TermId(2)));

    // Delete terms for site 1's post
    term_repo
        .delete_all_terms_for_object(&test_db, &site1, rowid1)
        .unwrap();

    // Verify site 1 has no terms
    let site1_terms_after = term_repo
        .get_all_terms_for_object(&test_db, &site1, rowid1)
        .unwrap();
    assert_eq!(site1_terms_after.len(), 0);

    // Verify site 2 still has its terms (not affected by site 1's deletion)
    let site2_terms_after = term_repo
        .get_all_terms_for_object(&test_db, &site2, rowid2)
        .unwrap();
    let site2_categories_after = site2_terms_after.get(&TaxonomyType::Category).unwrap();
    assert_eq!(
        site2_categories_after.len(),
        2,
        "Site 2's terms should not be affected by Site 1's term deletion"
    );
}
