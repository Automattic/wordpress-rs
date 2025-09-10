#[test]
fn wp_deserialize_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/wp_deserialize/pass/*.rs");
    t.compile_fail("tests/wp_deserialize/fail/*.rs");
}

#[test]
fn wp_derive_params_field_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/wp_derive_params_field/pass/*.rs");
    t.compile_fail("tests/wp_derive_params_field/fail/*.rs");
}
