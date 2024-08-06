use std::fs::metadata;
use std::path::Path;
use std::{fs, io};

pub async fn restore_wp_content_plugins() {
    println!("Restoring wp-content/plugins..");

    let wp_content_path =
        &std::env::var("WP_CONTENT_PATH").unwrap_or("/var/www/html/wp-content".to_string());

    let plugins_folder = &format!("{}/plugins", wp_content_path);
    let plugins_backup_folder = &format!("{}/plugins-backup", wp_content_path);

    std::fs::remove_dir_all(plugins_folder).expect("Failed to remove old plugins");

    recursive_copy_dir(plugins_backup_folder, plugins_folder)
        .expect("Failed to restore wp-content/plugins");
}

fn recursive_copy_dir(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    copy_ownership(&src, &dst)?;
    for entry in fs::read_dir(&src)? {
        let entry = entry?;
        let entry_path = &entry.path();
        if entry.file_type()?.is_dir() {
            recursive_copy_dir(entry_path, dst.as_ref().join(entry.file_name()))?;
        } else {
            let dest_path = &dst.as_ref().join(entry.file_name());
            fs::copy(entry_path, dest_path)?;
            copy_ownership(entry_path, dest_path)?;
        }
    }
    Ok(())
}

fn copy_ownership(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    if cfg!(unix) {
        use std::os::unix::fs::MetadataExt;
        let metadata = metadata(src)?;
        std::os::unix::fs::chown(dst, Some(metadata.uid()), Some(metadata.gid()))
    } else {
        panic!("Integration tests are only supported in Unix");
    }
}
