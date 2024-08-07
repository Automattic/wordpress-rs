use clap::{Parser, Subcommand};
use wp_api_integration_tests::fs_utils::restore_wp_content_plugins;

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
