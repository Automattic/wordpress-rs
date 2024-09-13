use std::{
    env,
    error::Error,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

fn main() -> Result<(), Box<dyn Error>> {
    generate_test_credentials_file()
}

fn generate_test_credentials_file() -> Result<(), Box<dyn Error>> {
    // Tell Cargo to rerun if the test credentials file changes
    println!("cargo::rerun-if-changed=../test_credentials");

    let out_dir = env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("generated_test_credentials.rs");
    let mut buf_writer = BufWriter::new(File::create(dest_path)?);

    let instance = if let Ok(file) = fs::File::open("../test_credentials.json") {
        let fields = serde_json::from_reader::<File, serde_json::Value>(file)
            .expect("test_credentials.json should be a valid JSON file")
            .as_object()
            .expect("test_credentials.json should be a valid JSON Object")
            .into_iter()
            .map(|(k, v)| format!("{}: {}.to_string(),", k, v))
            .collect::<Vec<String>>()
            .join("\n");
        format!("TestCredentials {{ {} }}", fields)
    } else {
        "TestCredentials::default()".to_string()
    };
    let generated_content = format!(
        r#"
            impl TestCredentials {{
                pub fn instance() -> Self {{
                    {}
                }}
            }}
        "#,
        instance
    );

    write!(buf_writer, "{}", generated_content)?;

    Ok(())
}
