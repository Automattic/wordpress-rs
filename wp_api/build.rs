use std::{
    env,
    error::Error,
    fs::{read_to_string, File},
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
    let mut buf_writer = BufWriter::new(File::create(&dest_path)?);

    if let Ok(file_contents) = read_to_string("../test_credentials") {
        let lines: Vec<&str> = file_contents.lines().collect();

        if !lines.is_empty() {
            let generated_content = format!(
                r#"
pub const TEST_CREDENTIALS_SITE_URL: &str = "{}";
pub const TEST_CREDENTIALS_ADMIN_USERNAME: &str = "{}";
pub const TEST_CREDENTIALS_ADMIN_PASSWORD: &str = "{}";
pub const TEST_CREDENTIALS_SUBSCRIBER_USERNAME: &str = "{}";
pub const TEST_CREDENTIALS_SUBSCRIBER_PASSWORD: &str = "{}";
            "#,
                lines[0], lines[1], lines[2], lines[3], lines[4]
            );

            write!(buf_writer, "{}", generated_content.trim())?;
        }
    } else {
        write!(buf_writer, "")?;
    }

    Ok(())
}
