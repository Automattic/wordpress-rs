#[test]
fn tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/wp_deserialize/pass/*.rs");
    t.compile_fail("tests/wp_deserialize/fail/*.rs");
}
