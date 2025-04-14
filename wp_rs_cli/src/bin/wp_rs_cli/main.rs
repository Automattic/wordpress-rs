use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use csv::Writer;
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use wp_api::{
    login::{
        login_client::WpLoginClient,
        url_discovery::{AutoDiscoveryAttemptType, AutoDiscoveryResult},
    },
    middleware::WpApiMiddlewarePipeline,
    reqwest_request_executor::ReqwestRequestExecutor,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    DiscoverLoginUrl {
        site: String,
    },
    BatchTestAutodiscovery {
        input_file: String,
        output_file: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::DiscoverLoginUrl { site } => {
            discover_login_url(site).await?;
        }
        Commands::BatchTestAutodiscovery {
            input_file,
            output_file,
        } => {
            batch_test_autodiscovery(input_file, output_file).await?;
        }
    }

    Ok(())
}

async fn discover_login_url(site: String) -> Result<()> {
    let intro = format!("Discovering login URL for {}", site).blue();
    println!("{}", intro);

    let result = test_url(site).await;

    if let Some(attempt) = result.find_successful() {
        let login_url = attempt
            .clone()
            .api_discovery_result
            .expect("This is the successful attempt")
            .api_details
            .find_application_passwords_authentication_url()
            .expect("Login URL must be found in a successful attempt");

        let success = format!("Login URL found: {}", login_url).green();
        println!("{}", success);
        return Ok(());
    }

    if let Some(error) = result
        .user_input_attempt()
        .api_discovery_result
        .clone()
        .err()
    {
        let error = format!("Error: {}", error).red();
        println!("{}", error);
        return Ok(());
    }

    Ok(())
}

#[derive(Debug, serde::Deserialize)]
struct BatchTestRow {
    #[serde(rename = "URL")]
    url: String,
}

async fn batch_test_autodiscovery(input_file: String, output_file: String) -> Result<()> {
    let intro = format!("Batch testing autodiscovery for {}", input_file).blue();
    println!("{}", intro);
    let mut writer = Writer::from_path(output_file)?;

    let mut s = stream::FuturesUnordered::new();

    for row in parse_input_file(input_file)? {
        s.push(test_url(row.url.clone()));
    }

    let count = format!("Scheduled {} URLs to test", s.len()).blue();
    println!("{}", count);

    while let Some(result) = s.next().await {
        // println!("{:?}", result);
        let outcome = result.is_successful().to_string();
        let site_url = result.user_input_attempt().attempt_site_url.to_string();

        if result.is_successful() {
            writer.write_record(&[outcome, site_url, "".to_string()])?;
        } else {
            if let Some(attempt) = result
                .attempts
                .get(&AutoDiscoveryAttemptType::AutoStrippedHttps)
            {
                if let Some(error) = attempt.api_discovery_result.as_ref().err() {
                    writer.write_record(&[outcome, site_url, rewrite_error(error.to_string())])?;
                    continue;
                }
            }

            let attempt = result.user_input_attempt();
            if let Some(error) = attempt.api_discovery_result.as_ref().err() {
                writer.write_record(&[outcome, site_url, rewrite_error(error.to_string())])?;
            }
        }
        writer.flush()?;
    }

    Ok(())
}

fn rewrite_error(error: String) -> String {
    if error.contains("Api root link header not found") {
        "Unhelpful error".to_string()
    } else {
        error.to_string()
    }
}

fn parse_input_file(input_file: String) -> Result<Vec<BatchTestRow>> {
    let mut rdr = csv::Reader::from_path(input_file)?;
    let rows: Vec<BatchTestRow> = rdr
        .deserialize::<BatchTestRow>()
        .filter_map(|r| r.ok())
        .collect::<Vec<BatchTestRow>>();
    Ok(rows)
}

async fn test_url(url: String) -> AutoDiscoveryResult {
    let request_executor = Arc::new(ReqwestRequestExecutor::new_with_default_timeout(false));
    let login_client = WpLoginClient::new(
        request_executor,
        Arc::new(WpApiMiddlewarePipeline {
            middlewares: vec![],
        }),
    );

    println!("Testing {}", url);
    login_client.api_discovery(url).await
}
