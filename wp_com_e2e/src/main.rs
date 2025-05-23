use anyhow::Result;
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use oauth2_tests::Oauth2Test;
use std::sync::Arc;
use support_eligibility_test::SupportEligibilityTest;
use support_tickets_test::SupportTicketsTest;
use wp_api::{
    WpApiClientDelegate, WpAppNotifier,
    auth::{WpAuthentication, WpAuthenticationProvider},
    middleware::WpApiMiddlewarePipeline,
    reqwest_request_executor::ReqwestRequestExecutor,
    wp_com::client::WpComApiClient,
};

mod oauth2_tests;
mod support_eligibility_test;
mod support_tickets_test;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Test {
        #[arg(short = 't', long = "token", env = "WP_COM_API_KEY")]
        token: String,
    },
}

#[async_trait]
trait Testable {
    async fn test(&self) -> Result<(), anyhow::Error>;
}

#[derive(Debug)]
pub struct EmptyAppNotifier;

#[async_trait]
impl WpAppNotifier for EmptyAppNotifier {
    async fn requested_with_invalid_authentication(&self) {
        // no-op
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Test { token } => {
            let delegate = WpApiClientDelegate {
                auth_provider: WpAuthenticationProvider::static_with_auth(
                    WpAuthentication::Bearer {
                        token: token.clone(),
                    },
                )
                .into(),
                request_executor: Arc::new(ReqwestRequestExecutor::default()),
                middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
                app_notifier: Arc::new(EmptyAppNotifier),
            };

            let client = WpComApiClient::new(delegate);

            Oauth2Test {
                client: &client,
                token: &token,
            }
            .test()
            .await?;
            SupportTicketsTest { client: &client }.test().await?;
            SupportEligibilityTest { client: &client }.test().await?;
        }
    }

    Ok(())
}
