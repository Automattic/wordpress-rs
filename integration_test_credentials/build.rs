use std::{
    env,
    error::Error,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("generated_test_credentials.rs");
    let mut buf_writer = BufWriter::new(File::create(dest_path)?);

    generate_test_credentials("test_credentials", "TestCredentials", &mut buf_writer)?;
    generate_test_credentials(
        "wp_com_test_credentials",
        "WpComTestCredentials",
        &mut buf_writer,
    )
}

fn generate_test_credentials(
    file_name: &str,
    test_credentials_type: &str,
    buf_writer: &mut BufWriter<File>,
) -> Result<(), Box<dyn Error>> {
    let test_credential_json_file_path = format!("../{file_name}.json");
    // Tell Cargo to rerun if the test credentials file changes
    println!("cargo::rerun-if-changed={test_credential_json_file_path}");

    let instance = if let Ok(file) = fs::File::open(&test_credential_json_file_path) {
        let fields = serde_json::from_reader::<File, serde_json::Value>(file)
            .expect("{test_credential_json_file_path} should be a valid JSON file")
            .as_object()
            .expect("{test_credential_json_file_path} should be a valid JSON Object")
            .into_iter()
            .map(|(k, v)| format!("{k}: {v},"))
            .collect::<Vec<String>>()
            .join("\n");
        format!("{test_credentials_type} {{ {fields} }}")
    } else {
        format!("{test_credentials_type}::default()")
    };
    let generated_content = format!(
        r#"
            impl {test_credentials_type} {{
                pub fn instance() -> Self {{
                    {instance}
                }}
            }}
        "#
    );

    write!(buf_writer, "{generated_content}")?;

    Ok(())
}
