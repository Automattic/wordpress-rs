use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, launch, routes, Responder};
use sqlx::Executor;
use sqlx::{mysql::MySqlConnectOptions, ConnectOptions};
use std::fs;
use std::fs::metadata;
use std::io;
use std::path::Path;
use wp_cli::{WpCliSiteSettings, WpCliUser, WpCliUserMeta};

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
        inner_restore_wp_db()
            .await
            .map_err(|e| Error::AsString(e.to_string()))?
    }
    Ok(Status::Ok)
}

#[get("/restore-wp-db")]
async fn restore_wp_db() -> Result<Status, Error> {
    inner_restore_wp_db()
        .await
        .map_err(|e| Error::AsString(e.to_string()))
        .map(|_| Status::Ok)
}

#[get("/restore-wp-content-plugins")]
async fn restore_wp_content_plugins() -> Status {
    inner_restore_wp_content_plugins().await;
    Status::Ok
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![restore_wp_db])
        .mount("/", routes![restore_wp_server])
        .mount("/", routes![restore_wp_content_plugins])
        .mount("/wp-cli/", routes![wp_cli_site_settings])
        .mount("/wp-cli/", routes![wp_cli_user])
        .mount("/wp-cli/", routes![wp_cli_users])
        .mount("/wp-cli/", routes![wp_cli_user_meta])
}

pub async fn inner_restore_wp_db() -> Result<(), sqlx::Error> {
    let db_dump_path = format!("{}/dump.sql", TEST_SITE_WP_CONTENT_PATH);

    let options = MySqlConnectOptions::new()
        .host("database")
        .username("wordpress")
        .password("wordpress")
        .database("wordpress");

    println!("Restoring WordPressDB from {:?}", db_dump_path);
    let mut conn = MySqlConnectOptions::connect(&options).await?;

    let db_schema = std::fs::read_to_string(db_dump_path).expect("Failed to read SQL dump");

    let _ = &mut conn.execute(db_schema.as_str()).await?;

    println!("Restored WordPressDB!");
    Ok(())
}

pub async fn inner_restore_wp_content_plugins() {
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
