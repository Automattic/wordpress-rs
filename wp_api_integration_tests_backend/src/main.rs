use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, launch, routes, Responder};
use std::fs;
use std::fs::metadata;
use std::io;
use std::path::Path;
use wp_cli::{WpCliPost, WpCliPostListArguments, WpCliSiteSettings, WpCliUser, WpCliUserMeta};

pub(crate) const TEST_SITE_WP_CONTENT_PATH: &str = "/var/www/html/wp-content";

#[derive(Responder)]
enum Error {
    #[response(status = 500)]
    AsString(String),
}

#[get("/site-settings")]
fn wp_cli_site_settings() -> Result<Json<WpCliSiteSettings>, Error> {
    WpCliSiteSettings::list()
        .map(Json)
        .map_err(|e| Error::AsString(e.to_string()))
}

#[get("/posts?<post_status>")]
fn wp_cli_posts(post_status: Option<String>) -> Result<Json<Vec<WpCliPost>>, Error> {
    WpCliPost::list(Some(WpCliPostListArguments { post_status }))
        .map(Json)
        .map_err(|e| Error::AsString(e.to_string()))
}

#[get("/user?<user_id>")]
fn wp_cli_user(user_id: i64) -> Result<Json<WpCliUser>, Error> {
    WpCliUser::get(user_id)
        .map(Json)
        .map_err(|e| Error::AsString(e.to_string()))
}

#[get("/users")]
fn wp_cli_users() -> Result<Json<Vec<WpCliUser>>, Error> {
    WpCliUser::list()
        .map(Json)
        .map_err(|e| Error::AsString(e.to_string()))
}

#[get("/user-meta?<user_id>")]
fn wp_cli_user_meta(user_id: i64) -> Result<Json<Vec<WpCliUserMeta>>, Error> {
    WpCliUserMeta::list(user_id)
        .map(Json)
        .map_err(|e| Error::AsString(e.to_string()))
}

#[get("/restore?<db>&<plugins>")]
async fn restore_wp_server(db: bool, plugins: bool) -> Result<Status, Error> {
    if plugins {
        inner_restore_wp_content_plugins().await;
    }
    if db {
        let output = wp_cli::restore_db();
        if !output.status.success() {
            return Err(Error::AsString(format!(
                "Failed to restore db: {:#?}",
                output
            )));
        }
    }
    Ok(Status::Ok)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![restore_wp_server])
        .mount("/wp-cli/", routes![wp_cli_site_settings])
        .mount("/wp-cli/", routes![wp_cli_posts])
        .mount("/wp-cli/", routes![wp_cli_user])
        .mount("/wp-cli/", routes![wp_cli_users])
        .mount("/wp-cli/", routes![wp_cli_user_meta])
}

async fn inner_restore_wp_content_plugins() {
    println!("Restoring wp-content/plugins");

    let plugins_folder = &format!("{}/plugins", TEST_SITE_WP_CONTENT_PATH);
    let plugins_backup_folder = &format!("{}/plugins-backup", TEST_SITE_WP_CONTENT_PATH);

    std::fs::remove_dir_all(plugins_folder).expect("Failed to remove old plugins");

    recursive_copy_dir(plugins_backup_folder, plugins_folder)
        .expect("Failed to restore wp-content/plugins");

    println!("Restored wp-content/plugins!");
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
