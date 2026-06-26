use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::File,
    process::{Command, Stdio},
};

mod wp_cli_categories;
mod wp_cli_comments;
mod wp_cli_pages;
mod wp_cli_posts;
mod wp_cli_settings;
mod wp_cli_tags;
mod wp_cli_users;

pub use wp_cli_categories::*;
pub use wp_cli_comments::*;
pub use wp_cli_pages::*;
pub use wp_cli_posts::*;
pub use wp_cli_settings::*;
pub use wp_cli_tags::*;
pub use wp_cli_users::*;

const BACKUP_PATH: &str = "/var/www/html/wp-content/dump.sql";

pub fn restore_db() -> std::process::Output {
    Command::new("mariadb")
        // Disable SSL to avoid connection errors
        .arg("--skip-ssl")
        // Host flag
        .arg("-h")
        // MySQL/MariaDB container hostname
        .arg("database")
        // Username flag
        .arg("-u")
        // Database username
        .arg("wordpress")
        // Database password
        .arg("-pwordpress")
        // Database name to connect to
        .arg("wordpress")
        // Pipe SQL dump file contents to stdin
        .stdin(Stdio::from(
            File::open(BACKUP_PATH).expect("Failed to open backup file"),
        ))
        .output()
        .expect("Failed to restore db")
}

/// Reads the site's current `permalink_structure` option. An empty string means
/// "Plain" permalinks, the case where WordPress advertises the REST API root in
/// the `…/index.php?rest_route=/` form rather than `…/wp-json/`.
pub fn get_permalink_structure() -> String {
    // Read the raw value (no `--format`): empty output for "Plain", otherwise the
    // structure string. `rewrite`/`option` writes below reject `--format`, so this
    // can't share `run_wp_cli_command`.
    let output = run_wp_cli_command_raw(["option", "get", "permalink_structure"]);
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Sets the permalink structure and flushes the rewrite rules in one step.
/// Passing an empty string selects "Plain" permalinks. Use this (not a bare
/// `option update`) so the cached `rewrite_rules` option stays consistent with
/// the new structure.
pub fn set_permalink_structure(structure: &str) -> std::process::Output {
    run_wp_cli_command_raw(["rewrite", "structure", structure])
}

fn run_wp_cli_command<I, S>(args: I) -> std::process::Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut c = wp_cli_command();
    c.arg("--format=json").args(args);
    println!("Running wp_cli command: {c:#?}");
    c.output().expect("Failed to run wp-cli command")
}

/// Like [`run_wp_cli_command`] but without `--format=json`, for commands such as
/// `rewrite structure` that reject the `--format` parameter.
fn run_wp_cli_command_raw<I, S>(args: I) -> std::process::Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut c = wp_cli_command();
    c.args(args);
    println!("Running wp_cli command: {c:#?}");
    c.output().expect("Failed to run wp-cli command")
}

fn wp_cli_command() -> Command {
    let mut c = Command::new("wp");
    c.arg("--allow-root")
        .arg("--http=http://localhost")
        .arg("--path=/var/www/html");
    c
}

pub(crate) trait AsWpCliArguments {
    fn as_wp_cli_arguments(&self) -> Option<String>;
}

impl AsWpCliArguments for HashMap<&'static str, &String> {
    fn as_wp_cli_arguments(&self) -> Option<String> {
        let mut s = String::new();
        self.iter().for_each(|(k, v)| {
            s.push_str(format!("--{k}={v}").as_str());
        });
        if s.is_empty() { None } else { Some(s) }
    }
}
