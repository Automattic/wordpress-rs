use std::sync::Arc;
use wp_api::parsed_url::ParsedUrl;
use wp_api::request::endpoint::posts_endpoint::PostEndpointType;
use wp_api::wp_com::WpComSiteId;
use wp_mobile::filters::PostListFilter;
use wp_mobile_cache::WpApiCache;
use wp_mobile_cache::repository::sites::SiteRepository;
use wp_mobile_integration_tests::*;

/// Helper to count rows in a table filtered by db_site_id.
fn count_rows_for_site(cache: &WpApiCache, table: &str, db_site_id: i64) -> usize {
    cache.execute(|conn| {
        let sql = format!("SELECT COUNT(*) FROM {} WHERE db_site_id = ?", table);
        let mut stmt = conn.prepare(&sql).expect("Failed to prepare count query");
        let count: i64 = stmt
            .query_row([db_site_id], |row| row.get(0))
            .expect("Failed to count rows");
        count as usize
    })
}

#[tokio::test]
#[serial]
async fn test_remove_self_hosted_site_deletes_all_cached_data() {
    let ctx = create_test_context();
    let site_url = TestCredentials::instance().site_url.to_string();

    // Sync some posts to populate the cache
    let collection = ctx
        .service
        .posts()
        .create_post_metadata_collection_with_edit_context(
            PostEndpointType::Posts,
            PostListFilter::default(),
            10,
        );
    let result = collection.refresh().await;
    assert!(result.is_ok(), "refresh should succeed: {:?}", result.err());

    let items = collection
        .load_items()
        .await
        .expect("load_items should succeed");
    assert!(!items.is_empty(), "should have loaded some items");

    // Verify data exists in cache before removal
    let db_site_id = ctx.cache.execute(|conn| {
        let site = SiteRepository
            .select_self_hosted_site_by_url(conn, &site_url)
            .expect("select should succeed")
            .expect("site should exist");
        site.data.0.row_id.0
    });
    assert!(
        count_rows_for_site(&ctx.cache, "posts_edit_context", db_site_id) > 0,
        "posts should be cached before removal"
    );

    // Remove the site
    let removed = ctx
        .cache
        .remove_self_hosted_site(Arc::new(ParsedUrl::parse(&site_url).unwrap()))
        .expect("remove should succeed");
    assert!(removed, "should return true when site existed");

    // Verify all data is gone
    assert_eq!(
        count_rows_for_site(&ctx.cache, "posts_edit_context", db_site_id),
        0,
        "posts_edit_context should be empty after removal"
    );
    assert_eq!(
        count_rows_for_site(&ctx.cache, "list_metadata", db_site_id),
        0,
        "list_metadata should be empty after removal"
    );
    assert_eq!(
        count_rows_for_site(&ctx.cache, "entity_state", db_site_id),
        0,
        "entity_state should be empty after removal"
    );

    // Verify the site itself is gone
    ctx.cache.execute(|conn| {
        let site = SiteRepository
            .select_self_hosted_site_by_url(conn, &site_url)
            .expect("select should succeed");
        assert!(site.is_none(), "site should no longer exist");
    });
}

#[tokio::test]
#[serial]
async fn test_remove_non_existent_self_hosted_site_returns_false() {
    let ctx = create_test_context();

    let removed = ctx
        .cache
        .remove_self_hosted_site(Arc::new(
            ParsedUrl::parse("https://non-existent-site.example.com").unwrap(),
        ))
        .expect("remove should succeed");
    assert!(!removed, "should return false for non-existent site");
}

#[tokio::test]
#[serial]
async fn test_remove_non_existent_wordpress_com_site_returns_false() {
    let ctx = create_test_context();

    let removed = ctx
        .cache
        .remove_wordpress_com_site(WpComSiteId(99999999))
        .expect("remove should succeed");
    assert!(!removed, "should return false for non-existent site");
}

#[tokio::test]
#[serial]
async fn test_remove_site_preserves_other_sites_data() {
    let ctx = create_test_context();

    let site_url_1 = TestCredentials::instance().site_url.to_string();
    let site_url_2 = "https://other-site.example.com";

    let collection = ctx
        .service
        .posts()
        .create_post_metadata_collection_with_edit_context(
            PostEndpointType::Posts,
            PostListFilter::default(),
            10,
        );
    let result = collection.refresh().await;
    assert!(result.is_ok(), "refresh should succeed: {:?}", result.err());

    // Create site 2 (just the site entry, no actual data sync needed)
    let _db_site_2 = ctx.cache.execute(|conn| {
        use wp_mobile_cache::db_types::self_hosted_site::SelfHostedSite;
        let site = SelfHostedSite {
            url: site_url_2.to_string(),
            api_root: format!("{}/wp-json", site_url_2),
        };
        SiteRepository
            .upsert_self_hosted_site(conn, &site)
            .expect("upsert should succeed")
    });

    // Verify both sites exist
    let site_count = ctx.cache.execute(|conn| {
        SiteRepository
            .count_all_db_sites(conn)
            .expect("count should succeed")
    });
    assert_eq!(site_count, 2, "should have two sites");

    // Get site 1's db_site_id for verification
    let db_site_id_1 = ctx.cache.execute(|conn| {
        let site = SiteRepository
            .select_self_hosted_site_by_url(conn, &site_url_1)
            .expect("select should succeed")
            .expect("site should exist");
        site.data.0.row_id.0
    });

    let posts_before = count_rows_for_site(&ctx.cache, "posts_edit_context", db_site_id_1);
    assert!(posts_before > 0, "site 1 should have cached posts");

    // Remove site 2
    let removed = ctx
        .cache
        .remove_self_hosted_site(Arc::new(ParsedUrl::parse(site_url_2).unwrap()))
        .expect("remove should succeed");
    assert!(removed, "site 2 should have been removed");

    // Verify site 1's data is intact
    let posts_after = count_rows_for_site(&ctx.cache, "posts_edit_context", db_site_id_1);
    assert_eq!(
        posts_before, posts_after,
        "site 1's posts should be preserved after removing site 2"
    );

    // Verify site 1 still exists
    ctx.cache.execute(|conn| {
        let site = SiteRepository
            .select_self_hosted_site_by_url(conn, &site_url_1)
            .expect("select should succeed");
        assert!(site.is_some(), "site 1 should still exist");
    });

    // Verify site 2 is gone
    ctx.cache.execute(|conn| {
        let site = SiteRepository
            .select_self_hosted_site_by_url(conn, site_url_2)
            .expect("select should succeed");
        assert!(site.is_none(), "site 2 should no longer exist");
    });
}
