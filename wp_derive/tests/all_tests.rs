#[test]
fn tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/basic_wp_contextual.rs");
    t.pass("tests/basic_wp_contextual_field.rs");
    t.compile_fail("tests/error_missing_sparse_prefix_from_wp_contextual.rs");
    t.compile_fail("tests/error_missing_sparse_prefix_from_wp_contextual_field.rs");
    t.compile_fail("tests/error_empty_result.rs");
    t.compile_fail("tests/error_wp_contextual_field_without_wp_context.rs");
    t.compile_fail("tests/error_unexpected_wp_context_literal.rs");
    t.compile_fail("tests/error_unexpected_wp_context_meta_variant_path.rs");
    t.compile_fail("tests/error_unexpected_wp_context_meta_variant_name_value.rs");
    // Test syntax errors for WPContextualField & WPContext field attributes
}
