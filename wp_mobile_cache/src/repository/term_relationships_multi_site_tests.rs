//! Multi-site isolation tests for TermRelationshipRepository.
//!
//! These tests verify that term relationships are correctly isolated between sites.

use crate::test_fixtures::{TestContext, create_test_site, posts::PostBuilder, test_ctx};
use rstest::*;
use wp_api::{taxonomies::TaxonomyType, terms::TermId};

#[rstest]
fn test_term_relationships_isolated_by_site(mut test_ctx: TestContext) {
    let site2 = create_test_site(&test_ctx.conn, 2);

    // Insert post in site 1 with categories
    let post1 = PostBuilder::minimal()
        .with_id(100)
        .with_categories(vec![TermId(1), TermId(2)])
        .build();
    test_ctx
        .post_repo
        .upsert(&mut test_ctx.conn, &test_ctx.site, &post1)
        .unwrap();

    // Insert post in site 2 with same categories
    let post2 = PostBuilder::minimal()
        .with_id(200)
        .with_categories(vec![TermId(1), TermId(2)])
        .build();
    test_ctx
        .post_repo
        .upsert(&mut test_ctx.conn, &site2, &post2)
        .unwrap();

    // Verify site 1's terms using WordPress post ID
    let site1_terms = test_ctx
        .term_repo
        .get_all_terms_for_object(&test_ctx.conn, &test_ctx.site, 100)
        .unwrap();
    let site1_categories = site1_terms.get(&TaxonomyType::Category).unwrap();
    assert_eq!(site1_categories.len(), 2);
    assert!(site1_categories.contains(&TermId(1)));
    assert!(site1_categories.contains(&TermId(2)));

    // Verify site 2's terms using WordPress post ID
    let site2_terms = test_ctx
        .term_repo
        .get_all_terms_for_object(&test_ctx.conn, &site2, 200)
        .unwrap();
    let site2_categories = site2_terms.get(&TaxonomyType::Category).unwrap();
    assert_eq!(site2_categories.len(), 2);
    assert!(site2_categories.contains(&TermId(1)));
    assert!(site2_categories.contains(&TermId(2)));

    // Delete terms for site 1's post
    test_ctx
        .term_repo
        .delete_all_terms_for_object(&test_ctx.conn, &test_ctx.site, 100)
        .unwrap();

    // Verify site 1 has no terms
    let site1_terms_after = test_ctx
        .term_repo
        .get_all_terms_for_object(&test_ctx.conn, &test_ctx.site, 100)
        .unwrap();
    assert_eq!(site1_terms_after.len(), 0);

    // Verify site 2 still has its terms (not affected by site 1's deletion)
    let site2_terms_after = test_ctx
        .term_repo
        .get_all_terms_for_object(&test_ctx.conn, &site2, 200)
        .unwrap();
    let site2_categories_after = site2_terms_after.get(&TaxonomyType::Category).unwrap();
    assert_eq!(
        site2_categories_after.len(),
        2,
        "Site 2's terms should not be affected by Site 1's term deletion"
    );
}
