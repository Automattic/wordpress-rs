use std::{collections::HashMap, ffi::OsStr, process::Command};

mod wp_cli_categories;
mod wp_cli_comments;
mod wp_cli_posts;
mod wp_cli_settings;
mod wp_cli_tags;
mod wp_cli_users;

pub use wp_cli_categories::*;
pub use wp_cli_comments::*;
pub use wp_cli_posts::*;
pub use wp_cli_settings::*;
pub use wp_cli_tags::*;
pub use wp_cli_users::*;

const BACKUP_PATH: &str = "/var/www/html/wp-content/dump.sql";

pub fn restore_db() -> std::process::Output {
    run_wp_cli_command(["db", "import", BACKUP_PATH])
}

fn run_wp_cli_command<I, S>(args: I) -> std::process::Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut c = Command::new("wp");
    c.arg("--allow-root")
        .arg("--http=http://localhost")
        .arg("--path=/var/www/html")
        .arg("--format=json")
        .args(args);
    println!("Running wp_cli command: {:#?}", c);
    c.output().expect("Failed to run wp-cli command")
}

pub(crate) trait AsWpCliArguments {
    fn as_wp_cli_arguments(&self) -> Option<String>;
}

impl AsWpCliArguments for HashMap<&'static str, &String> {
    fn as_wp_cli_arguments(&self) -> Option<String> {
        let mut s = String::new();
        self.iter().for_each(|(k, v)| {
            s.push_str(format!("--{}={}", k, v).as_str());
        });
        if s.is_empty() { None } else { Some(s) }
    }
}
