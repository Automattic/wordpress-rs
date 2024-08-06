use clap::{Parser, Subcommand};
use fs_utils::restore_wp_content_plugins;

mod fs_utils;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.cmd {
        Commands::RestoreWpContentPlugins => restore_wp_content_plugins().await,
    }
}

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    RestoreWpContentPlugins,
}
