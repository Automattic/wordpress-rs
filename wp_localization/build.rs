use std::{collections::HashSet, env, error::Error, fs, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=localization");

    let localization_dir = Path::new("localization");
    let mut available_languages = Vec::new();

    if let Ok(entries) = fs::read_dir(localization_dir) {
        let ignore: HashSet<_> = HashSet::from_iter([".", "..", ".DS_Store"]);
        for entry in entries.flatten() {
            let lang_id = entry.file_name().to_string_lossy().to_string();
            if ignore.contains(lang_id.as_str()) {
                continue;
            }

            available_languages.push(format!("langid!(\"{}\")", lang_id));
        }
    }

    let code = format!(
        "static AVAILABLE_LANGUAGES: &[LanguageIdentifier] = &[{}];",
        available_languages.join(", "),
    );

    let out_dir = env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("generated_wp_locale.rs");

    fs::write(dest_path, code)?;

    Ok(())
}
