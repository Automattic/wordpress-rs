use integration_test_credentials::WpComTestCredentials;
use libtest_mimic::{Arguments, Trial, run};
use std::env;
use std::sync::Arc;

mod api_root_tests;
mod context;
mod domains_tests;
mod languages_tests;
mod me_tests;
mod products_tests;
mod sites_tests;
mod stats_city_views_tests;
mod stats_country_views_tests;
mod stats_emails_summary_tests;
mod stats_insights_tests;
mod stats_referrers_tests;
mod stats_region_views_tests;
mod stats_subscribers_tests;
mod stats_summary_tests;
mod stats_top_authors_tests;
mod stats_top_posts_tests;
mod subscribers_by_user_type_tests;
mod support_bot_tests;
mod support_eligibility_test;
mod support_tickets_test;
mod transactions_tests;
mod unified_conversations_tests;
mod wp_service_tests;

use context::TestContext;

fn main() {
    let token = env::var("WP_COM_API_KEY").unwrap_or_else(|_| {
        let creds = WpComTestCredentials::instance();
        if creds.bearer_token.is_empty() {
            panic!(
                "WP_COM_API_KEY environment variable must be set, or wp_com_test_credentials.json must exist"
            );
        }
        creds.bearer_token.to_string()
    });

    let args = Arguments::from_args();
    let ctx = Arc::new(TestContext::new(token));

    let tests = collect_tests(Arc::clone(&ctx));

    run(&args, tests).exit();
}

fn collect_tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut tests = vec![];
    tests.extend(api_root_tests::tests(Arc::clone(&ctx)));
    tests.extend(domains_tests::tests(Arc::clone(&ctx)));
    tests.extend(languages_tests::tests(Arc::clone(&ctx)));
    tests.extend(me_tests::tests(Arc::clone(&ctx)));
    tests.extend(stats_city_views_tests::tests(Arc::clone(&ctx)));
    tests.extend(stats_country_views_tests::tests(Arc::clone(&ctx)));
    tests.extend(stats_insights_tests::tests(Arc::clone(&ctx)));
    tests.extend(stats_referrers_tests::tests(Arc::clone(&ctx)));
    tests.extend(stats_region_views_tests::tests(Arc::clone(&ctx)));
    tests.extend(stats_summary_tests::tests(Arc::clone(&ctx)));
    tests.extend(stats_top_authors_tests::tests(Arc::clone(&ctx)));
    tests.extend(sites_tests::tests(Arc::clone(&ctx)));
    tests.extend(support_bot_tests::tests(Arc::clone(&ctx)));
    tests.extend(support_eligibility_test::tests(Arc::clone(&ctx)));
    tests.extend(support_tickets_test::tests(Arc::clone(&ctx)));
    tests.extend(unified_conversations_tests::tests(Arc::clone(&ctx)));
    tests.extend(stats_emails_summary_tests::tests(Arc::clone(&ctx)));
    tests.extend(stats_subscribers_tests::tests(Arc::clone(&ctx)));
    tests.extend(stats_top_posts_tests::tests(Arc::clone(&ctx)));
    tests.extend(subscribers_by_user_type_tests::tests(Arc::clone(&ctx)));
    tests.extend(products_tests::tests(Arc::clone(&ctx)));
    tests.extend(transactions_tests::tests(Arc::clone(&ctx)));
    tests.extend(wp_service_tests::tests(Arc::clone(&ctx)));
    tests
}
