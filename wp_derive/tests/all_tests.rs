#[test]
fn tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/basic_wp_contextual.rs");
    t.pass("tests/basic_wp_contextual_field.rs");
    t.compile_fail("tests/error_missing_sparse_prefix_from_wp_contextual.rs");
}
