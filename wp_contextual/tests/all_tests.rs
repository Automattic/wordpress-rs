#[test]
fn tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/basic_wp_contextual.rs");
    t.pass("tests/basic_wp_contextual_field.rs");
    t.pass("tests/basic_wp_contextual_option.rs");
    t.pass("tests/wp_contextual_field_with_multiple_segments.rs");
    t.pass("tests/wp_contextual_field_with_inner_type.rs");
    t.compile_fail("tests/error_both_wp_contextual_field_and_wp_contextual_option.rs");
    t.compile_fail("tests/error_missing_sparse_prefix_from_wp_contextual.rs");
    t.compile_fail("tests/error_missing_sparse_prefix_from_wp_contextual_field.rs");
    t.compile_fail("tests/error_empty_result.rs");
    t.compile_fail("tests/error_unexpected_wp_context_ident.rs");
    t.compile_fail("tests/error_unexpected_wp_context_meta_variant_path.rs");
    t.compile_fail("tests/error_unexpected_wp_context_meta_variant_name_value.rs");
    t.compile_fail("tests/error_unexpected_wp_context_punct.rs");
    t.compile_fail("tests/error_unexpected_wp_context_literal.rs");
    t.compile_fail("tests/error_unexpected_wp_context_token.rs");
    t.compile_fail("tests/error_wp_contextual_field_without_wp_context.rs");
    t.compile_fail("tests/error_wp_contextual_not_a_struct.rs");
}
