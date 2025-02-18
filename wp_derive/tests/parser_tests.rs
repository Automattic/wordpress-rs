use std::{env, fs, io::Write};

#[test]
fn tests() {
    let t = trybuild::TestCases::new();

    // wp_deserialize
    t.pass("tests/wp_deserialize/pass/*.rs");
    t.compile_fail("tests/wp_deserialize/fail/*.rs");

    // wp_translations
    generate_test_translations();
    generate_translations_config_file();
    t.pass("tests/wp_translations/pass/*.rs");
}

const FTL: &str = r#"
key = { $emailCount }
"#;

fn generate_test_translations() {
    let file_path = format!("{}/test.ftl", path_used_by_trybuild());
    let mut file = fs::File::create(file_path).unwrap();
    let _ = file.write(FTL.as_bytes());
}

fn generate_translations_config_file() {
    let file_path = format!("{}/wp_translations.toml", path_used_by_trybuild());
    let mut file = fs::File::create(file_path).unwrap();
    let _ = file.write(b"path = \"test.ftl\"");
}

fn path_used_by_trybuild() -> String {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{manifest_dir}/../target/tests/trybuild/wp_derive")
}
