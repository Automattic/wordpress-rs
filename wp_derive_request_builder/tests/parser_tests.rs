// Don't run parser tests if the request builders are being generated because request
// builders include `wp_api` crate specific types.
#[test]
#[cfg_attr(feature = "generate_request_builder", ignore)]
fn tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/pass/*.rs");
    t.compile_fail("tests/fail/*.rs");
}
