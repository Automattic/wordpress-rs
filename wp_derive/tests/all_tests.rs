#[test]
fn tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/basic_wp_contextual.rs");
}
