use libtest_mimic::{Arguments, Trial, run};
use std::env;
use std::sync::Arc;

mod context;
mod languages_tests;
mod me_tests;
mod sites_tests;
mod support_bot_tests;
mod support_eligibility_test;
mod support_tickets_test;

use context::TestContext;

fn main() {
    let token =
        env::var("WP_COM_API_KEY").expect("WP_COM_API_KEY environment variable must be set");

    let args = Arguments::from_args();
    let ctx = Arc::new(TestContext::new(token));

    let tests = collect_tests(Arc::clone(&ctx));

    run(&args, tests).exit();
}

fn collect_tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut tests = vec![];
    tests.extend(languages_tests::tests(Arc::clone(&ctx)));
    tests.extend(me_tests::tests(Arc::clone(&ctx)));
    tests.extend(sites_tests::tests(Arc::clone(&ctx)));
    tests.extend(support_bot_tests::tests(Arc::clone(&ctx)));
    tests.extend(support_eligibility_test::tests(Arc::clone(&ctx)));
    tests.extend(support_tickets_test::tests(Arc::clone(&ctx)));
    tests
}
