use std::{
    collections::HashMap,
    env,
    ffi::{OsStr, OsString},
    fs::File,
    path::PathBuf,
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

/// Where the test site's WordPress files live.
///
/// Defaults to the document root of the `wordpress` Docker image. Set `WP_TEST_SITE_PATH` when
/// the site is installed somewhere else, as `scripts/native-test-server.sh` does.
pub fn test_site_path() -> PathBuf {
    env::var_os("WP_TEST_SITE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/www/html"))
}

/// Hostname of the test site's database server. Defaults to the `database` service from
/// `docker-compose.yml`; override it with `WP_TEST_DB_HOST`.
fn test_db_host() -> String {
    env::var("WP_TEST_DB_HOST").unwrap_or_else(|_| "database".to_string())
}

/// Port of the test site's database server. Override it with `WP_TEST_DB_PORT`.
fn test_db_port() -> String {
    env::var("WP_TEST_DB_PORT").unwrap_or_else(|_| "3306".to_string())
}

pub fn restore_db() -> std::process::Output {
    let backup_path = test_site_path().join("wp-content/dump.sql");
    Command::new("mariadb")
        // Disable SSL to avoid connection errors
        .arg("--skip-ssl")
        // Host flag
        .arg("-h")
        // MySQL/MariaDB hostname
        .arg(test_db_host())
        // Port flag
        .arg("-P")
        // MySQL/MariaDB port
        .arg(test_db_port())
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
            File::open(backup_path).expect("Failed to open backup file"),
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
    let mut path_arg = OsString::from("--path=");
    path_arg.push(test_site_path());
    c.arg("--allow-root")
        .arg("--http=http://localhost")
        .arg(path_arg);
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
