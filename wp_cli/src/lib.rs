use std::{ffi::OsStr, process::Command};

mod wp_cli_settings;
mod wp_cli_users;

pub use wp_cli_settings::*;
pub use wp_cli_users::*;

fn run_wp_cli_command<I, S>(args: I) -> std::process::Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut c = Command::new("wp");
    c.arg("--http=http://localhost")
        .arg("--path=/var/www/html")
        .arg("--format=json")
        .args(args);
    println!("Running wp_cli command: {:#?}", c);
    c.output().expect("Failed to run wp-cli command")
}
