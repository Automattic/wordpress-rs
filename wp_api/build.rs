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
    let mut buf_writer = BufWriter::new(File::create(dest_path)?);
    let generated_content = TestCredentials::from_raw_test_credentials_file("../test_credentials")
        .unwrap_or_default()
        .generated_test_credentials_file_content();
    write!(buf_writer, "{}", generated_content)?;

    Ok(())
}

#[derive(Debug, Default)]
struct TestCredentials {
    site_url: String,
    admin_username: String,
    admin_password: String,
    subscriber_username: String,
    subscriber_password: String,
}

impl TestCredentials {
    fn from_raw_test_credentials_file(file_path: &str) -> Option<Self> {
        if let Ok(file_contents) = read_to_string(file_path) {
            let lines: Vec<&str> = file_contents.lines().collect();

            if !lines.is_empty() {
                return Some(Self {
                    site_url: lines[0].to_string(),
                    admin_username: lines[1].to_string(),
                    admin_password: lines[2].to_string(),
                    subscriber_username: lines[3].to_string(),
                    subscriber_password: lines[4].to_string(),
                });
            }
        }
        None
    }

    fn generated_test_credentials_file_content(&self) -> String {
        format!(
            r#"
pub const TEST_CREDENTIALS_SITE_URL: &str = "{}";
pub const TEST_CREDENTIALS_ADMIN_USERNAME: &str = "{}";
pub const TEST_CREDENTIALS_ADMIN_PASSWORD: &str = "{}";
pub const TEST_CREDENTIALS_SUBSCRIBER_USERNAME: &str = "{}";
pub const TEST_CREDENTIALS_SUBSCRIBER_PASSWORD: &str = "{}";
"#,
            self.site_url,
            self.admin_username,
            self.admin_password,
            self.subscriber_username,
            self.subscriber_password
        )
        .trim()
        .to_string()
    }
}
