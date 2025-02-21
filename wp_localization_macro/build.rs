use std::{
    env,
    error::Error,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

const LOCALIZATION_FILE_PATH: &str = "../wp_localization/localization/en-US/main.ftl";

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo::rerun-if-changed={LOCALIZATION_FILE_PATH}");

    let contents = fs::read_to_string(LOCALIZATION_FILE_PATH)?;

    let out_dir = env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("generated_localization_contents.rs");
    let mut buf_writer = BufWriter::new(File::create(dest_path)?);
    let generated_content = format!(
        r#"
            const LOCALIZATION_CONTENTS: &str = "{}";
        "#,
        contents
    );

    write!(buf_writer, "{}", generated_content)?;

    Ok(())
}
