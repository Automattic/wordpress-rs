use std::sync::Arc;
use colored::Colorize;
use anyhow::Result;
use clap::{Parser, Subcommand};
use wp_api::{login::login_client::WpLoginClient, middleware::WpApiMiddlewarePipeline, reqwest_request_executor::ReqwestRequestExecutor};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    // /// WordPress site URL
    // #[arg(short, long)]
    // site: Option<String>,

    // /// Username for authentication
    // #[arg(short, long)]
    // username: Option<String>,

    // /// Password for authentication
    // #[arg(short, long)]
    // password: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    DiscoverLoginUrl {
        site: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::DiscoverLoginUrl { site } => {
            discover_login_url(site).await?;
        }
    }

    Ok(())
} 

async fn discover_login_url(site: String) -> Result<()> {
    let intro = format!("Discovering login URL for {}", site).blue();
    println!("{}", intro);

    let request_executor = Arc::new(ReqwestRequestExecutor::new(false));
    let login_client = WpLoginClient::new(request_executor, Arc::new(WpApiMiddlewarePipeline{ middlewares: vec![] }));
    let result = login_client.api_discovery(site).await;

    if let Some(attempt) = result.find_successful() {
        let login_url = attempt.clone().api_discovery_result
        .expect("This is the successful attempt")
        .api_details
        .find_application_passwords_authentication_url()
        .expect("Login URL must be found in a successful attempt");
    
        let success = format!("Login URL found: {}", login_url).green();
        println!("{}", success);
        return Ok(());
    } 
    
    if let Some(error) = result.user_input_attempt().api_discovery_result.clone().err() {
        let error = format!("Error: {}", error).red();
        println!("{}", error);
        return Ok(());
    }

    Ok(())
}