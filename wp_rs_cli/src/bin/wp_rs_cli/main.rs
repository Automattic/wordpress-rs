use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use csv::Writer;
use futures::stream::StreamExt;
use std::{fmt::Display, fs::File, sync::Arc};
use wp_api::{
    login::{
        login_client::WpLoginClient,
        url_discovery::{
            AutoDiscoveryAttemptFailure, FetchAndParseApiRootFailure, FindApiRootFailure,
        },
    },
    middleware::WpApiMiddlewarePipeline,
    reqwest_request_executor::ReqwestRequestExecutor,
};

const TOKIO_STREAM_SIZE: usize = 100;

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
    let login_client = build_login_client();

    match cli.command {
        Commands::DiscoverLoginUrl { site } => {
            discover_login_url(&login_client, site).await;
        }
        Commands::BatchTestAutodiscovery {
            input_file,
            output_file,
        } => {
            batch_test_autodiscovery(&login_client, input_file.as_str(), output_file).await?;
        }
    }

    Ok(())
}

async fn discover_login_url(login_client: &WpLoginClient, site: String) {
    let intro = format!("Discovering login URL for {}", site).blue();
    println!("{intro}");

    perform_api_discovery(login_client, site).await.log_result();
}

async fn batch_test_autodiscovery(
    login_client: &WpLoginClient,
    input_file: &str,
    output_file: String,
) -> Result<()> {
    let intro = format!("Batch testing autodiscovery for {}", input_file).blue();
    println!("{}", intro);

    let mut writer = Writer::from_path(output_file)?;
    let rows = parse_input_file(input_file)?;

    let count = format!("Scheduled {} URLs to test", rows.len()).blue();
    println!("{}", count);

    for r in batch_perform_autodiscovery(login_client, rows.iter()).await {
        r.write_as_csv_record(&mut writer)?;
    }
    writer.flush()?;

    Ok(())
}

async fn batch_perform_autodiscovery(
    login_client: &WpLoginClient,
    rows: impl Iterator<Item = &BatchTestRow>,
) -> Vec<SimplifiedDiscoveryResult> {
    let mut stream = tokio_stream::iter(rows)
        .map(|row| {
            let attempt_url = row.url.clone();
            perform_api_discovery(login_client, attempt_url.clone())
        })
        .buffer_unordered(TOKIO_STREAM_SIZE);

    {
        let mut results = vec![];
        while let Some(r) = stream.next().await {
            results.push(r);
        }
        results
    }
}

async fn perform_api_discovery(
    login_client: &WpLoginClient,
    url: String,
) -> SimplifiedDiscoveryResult {
    println!("Testing {}", url);
    match login_client
        .api_discovery(url.clone())
        .await
        .combined_result()
    {
        Ok(s) => {
            let login_url = s
                .api_details
                .find_application_passwords_authentication_url()
                .expect("Already confirmed auto discovery was successful");
            SimplifiedDiscoveryResult::success(url, LoginUrl(login_url))
        }
        Err(e) => SimplifiedDiscoveryResult::failure(url, e.clone()),
    }
}

fn build_login_client() -> WpLoginClient {
    let request_executor = Arc::new(ReqwestRequestExecutor::default());
    WpLoginClient::new(
        request_executor,
        Arc::new(WpApiMiddlewarePipeline {
            middlewares: vec![],
        }),
    )
}

fn parse_input_file(input_file: &str) -> Result<Vec<BatchTestRow>> {
    Ok(csv::Reader::from_path(input_file)?
        .deserialize::<BatchTestRow>()
        .filter_map(|r| r.ok())
        .collect())
}

#[derive(Debug, serde::Deserialize)]
struct BatchTestRow {
    #[serde(rename = "URL")]
    url: String,
}

#[derive(Debug)]
struct SimplifiedDiscoveryResult {
    attempt_site_url: String,
    result: Result<LoginUrl, AutoDiscoveryAttemptFailure>,
}

impl SimplifiedDiscoveryResult {
    fn success(attempt_site_url: String, login_url: LoginUrl) -> Self {
        Self {
            attempt_site_url,
            result: Ok(login_url),
        }
    }

    fn failure(attempt_site_url: String, error: AutoDiscoveryAttemptFailure) -> Self {
        Self {
            attempt_site_url,
            result: Err(error),
        }
    }

    fn write_as_csv_record(self, writer: &mut csv::Writer<File>) -> Result<()> {
        let (error_type, error_message) = self
            .result
            .as_ref()
            .err()
            .map(|e| (csv_error_type(e), e.to_string()))
            .unwrap_or(("".to_string(), "".to_string()));
        Ok(writer.write_record(&[
            self.result.is_ok().to_string(),
            self.attempt_site_url,
            error_type,
            error_message,
        ])?)
    }

    fn log_result(&self) {
        match &self.result {
            Ok(login_url) => {
                println!("{}", format!("Login URL found: {login_url}").green());
            }
            Err(error) => {
                println!("{}", format!("Error: {}", error).red());
            }
        }
    }
}

#[derive(Debug)]
struct LoginUrl(String);

impl Display for LoginUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn csv_error_type(failure: &AutoDiscoveryAttemptFailure) -> String {
    match failure {
        AutoDiscoveryAttemptFailure::ParseSiteUrl { .. } => "ParseSiteUrl".to_string(),
        AutoDiscoveryAttemptFailure::FindApiRoot {
            find_api_root_failure,
            ..
        } => match find_api_root_failure {
            FindApiRootFailure::FetchHomepage { .. } => "FetchHomepage".to_string(),
            FindApiRootFailure::ProbablyNotAWordPressSite => {
                "ProbablyNotAWordPressSite".to_string()
            }
            FindApiRootFailure::RestApiDisabled => "RestApiDisabled".to_string(),
        },
        AutoDiscoveryAttemptFailure::FetchAndParseApiRoot {
            fetch_and_parse_api_root_failure,
            ..
        } => match fetch_and_parse_api_root_failure {
            FetchAndParseApiRootFailure::FetchApiRoot { .. } => "FetchApiRoot".to_string(),
            FetchAndParseApiRootFailure::ParseApiRoot { .. } => "ParseApiRoot".to_string(),
            FetchAndParseApiRootFailure::WpError { error_code, .. } => {
                format!("WpError-{:#?}", error_code)
            }
            FetchAndParseApiRootFailure::ApplicationPasswordsNotSupported { reason, .. } => {
                format!("ApplicationPasswordsNotSupported-{:#?}", reason)
            }
        },
    }
}
