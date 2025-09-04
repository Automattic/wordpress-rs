use anyhow::{Result, anyhow};
use clap::{ArgGroup, Args, Parser, Subcommand};
use colored::Colorize;
use csv::Writer;
use futures::stream::StreamExt;
use std::{fmt::Display, fs::File, sync::Arc, time::Duration};
use url::Url;
use wp_api::{
    comments::CommentListParams,
    parsed_url::ParsedUrl,
    posts::{PostId, PostRetrieveParams},
    request::endpoint::WpOrgSiteApiUrlResolver,
    wp_com::{WpComBaseUrl, endpoint::WpComDotOrgApiUrlResolver},
};
use wp_api::{
    login::url_discovery::{
        AutoDiscoveryAttemptFailure, FetchAndParseApiRootFailure, FindApiRootFailure,
    },
    prelude::*,
};

const TOKIO_STREAM_SIZE: usize = 5;

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
    /// Fetch a single post and its comments
    FetchPost(FetchPostArgs),
}

#[derive(Debug, Args, Clone)]
struct AuthArgs {
    /// WordPress.com site (e.g. example.wordpress.com or numeric ID)
    #[arg(long)]
    wpcom_site: Option<String>,

    /// WordPress.org/Jetpack API root (must end with /wp-json)
    #[arg(long)]
    api_root: Option<String>,

    /// Bearer token for WordPress.com (fallback env: WP_BEARER_TOKEN)
    #[arg(long)]
    bearer: Option<String>,

    /// Application Password username for wp.org/Jetpack (fallback env: WP_USERNAME)
    #[arg(long)]
    username: Option<String>,

    /// Application Password for wp.org/Jetpack (fallback env: WP_APP_PASSWORD)
    #[arg(long)]
    password: Option<String>,
}

#[derive(Debug, Parser)]
#[command(group(
    ArgGroup::new("site_type")
        .args(["wpcom_site", "api_root", "url"]),
), group(
    ArgGroup::new("post_ref")
        .required(true)
        .args(["post_id", "url"]),
))]
struct FetchPostArgs {
    /// Common authentication parameters
    #[command(flatten)]
    auth: AuthArgs,

    /// Full post URL (alternative to --post-id)
    /// When provided, this URL is used to infer the site (wp.com) or autodiscover the API root (wp.org/Jetpack).
    #[arg(long)]
    url: Option<String>,

    /// The post ID to fetch
    #[arg(long, value_parser = parse_post_id)]
    post_id: Option<PostId>,

    /// Password for the post if it is password-protected
    #[arg(long)]
    post_password: Option<String>,

    /// Max items per page when fetching comments
    #[arg(long, default_value_t = 100)]
    per_page: u32,

    /// Output pretty-printed JSON
    #[arg(long, default_value_t = false)]
    pretty: bool,
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
        Commands::FetchPost(args) => {
            fetch_post_and_comments(args).await?;
        }
    }

    Ok(())
}

async fn discover_login_url(login_client: &WpLoginClient, site: String) {
    let intro = format!("Discovering login URL for {site}").blue();
    println!("{intro}");

    perform_api_discovery(login_client, site).await.log_result();
}

async fn batch_test_autodiscovery(
    login_client: &WpLoginClient,
    input_file: &str,
    output_file: String,
) -> Result<()> {
    let intro = format!("Batch testing autodiscovery for {input_file}").blue();
    println!("{intro}");

    let mut writer = Writer::from_path(output_file)?;
    let rows = parse_input_file(input_file)?;

    let count = format!("Scheduled {} URLs to test", rows.len()).blue();
    println!("{count}");

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
    println!("Testing {url}");
    match login_client
        .api_discovery(url.clone())
        .await
        .combined_result()
    {
        Ok(s) => SimplifiedDiscoveryResult::success(
            url,
            LoginUrl(s.application_passwords_authentication_url.clone()),
        ),
        Err(e) => SimplifiedDiscoveryResult::failure(url, e.clone()),
    }
}

fn build_login_client() -> WpLoginClient {
    let request_executor = Arc::new(ReqwestRequestExecutor::new(false, Duration::from_secs(60)));
    WpLoginClient::new_with_default_middleware_pipeline(request_executor)
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
                println!("{}", error.to_string().red());
            }
        }
    }
}

#[derive(Debug)]
struct LoginUrl(Arc<ParsedUrl>);

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
                format!("WpError-{error_code:#?}")
            }
            FetchAndParseApiRootFailure::ApplicationPasswordsNotSupported { reason, .. } => {
                format!("ApplicationPasswordsNotSupported-{reason:#?}")
            }
        },
    }
}

#[derive(Debug)]
enum SiteApiType {
    WpCom { site: String },
    WpOrg { api_root: Arc<ParsedUrl> },
}

impl SiteApiType {
    async fn detect_from_args(
        args: &AuthArgs,
        url: &Option<String>,
        request_executor: &Arc<ReqwestRequestExecutor>,
    ) -> Result<Self> {
        if let Some(site) = &args.wpcom_site {
            // Explicit WordPress.com site takes priority
            return Ok(SiteApiType::WpCom { site: site.clone() });
        }
        if let Some(api_root) = &args.api_root {
            // Explicit api_root takes priority for wp.org/Jetpack
            let parsed = ParsedUrl::try_from(api_root.as_str()).map_err(|_| {
                anyhow!("Invalid api_root URL: must be a valid URL ending with /wp-json")
            })?;
            return Ok(SiteApiType::WpOrg {
                api_root: Arc::new(parsed),
            });
        }
        if let Some(url) = url {
            // Derive from URL if possible
            if let Ok(u) = Url::parse(url.as_str()) {
                let host = u.host_str().unwrap_or("");
                if host.ends_with(".wordpress.com") {
                    return Ok(SiteApiType::WpCom {
                        site: host.to_string(),
                    });
                }

                // Attempt autodiscovery of API root from URL
                let login_client =
                    WpLoginClient::new_with_default_middleware_pipeline(request_executor.clone());
                match login_client
                    .api_discovery(url.clone())
                    .await
                    .combined_result()
                    .cloned()
                {
                    Ok(success) => Ok(SiteApiType::WpOrg {
                        api_root: success.api_root_url,
                    }),
                    Err(_) => Err(anyhow!(
                        "Could not autodiscover API root from URL. Please provide --api-root explicitly."
                    )),
                }
            } else {
                Err(anyhow!("Invalid URL; could not parse"))
            }
        } else {
            Err(anyhow!(
                "Provide either --wpcom-site, or --api-root, or a wordpress.com URL"
            ))
        }
    }

    fn api_url_resolver(&self) -> Arc<dyn ApiUrlResolver> {
        match self {
            SiteApiType::WpCom { site } => Arc::new(WpComDotOrgApiUrlResolver::new(
                site.clone(),
                WpComBaseUrl::Production,
            )),
            SiteApiType::WpOrg { api_root } => {
                Arc::new(WpOrgSiteApiUrlResolver::new(api_root.clone()))
            }
        }
    }

    fn auth_provider(&self, args: &AuthArgs) -> Result<Arc<WpAuthenticationProvider>> {
        match self {
            SiteApiType::WpCom { .. } => {
                let token = env_or_arg(&args.bearer, "WP_BEARER_TOKEN").ok_or_else(|| {
                    anyhow!("Missing bearer token. Provide --bearer or set WP_BEARER_TOKEN")
                })?;
                Ok(Arc::new(WpAuthenticationProvider::static_with_auth(
                    WpAuthentication::Bearer { token },
                )))
            }
            SiteApiType::WpOrg { .. } => {
                let username = env_or_arg(&args.username, "WP_USERNAME").ok_or_else(|| {
                    anyhow!("Missing username. Provide --username or set WP_USERNAME")
                })?;
                let password = env_or_arg(&args.password, "WP_APP_PASSWORD").ok_or_else(|| {
                    anyhow!(
                        "Missing application password. Provide --password or set WP_APP_PASSWORD"
                    )
                })?;
                Ok(Arc::new(
                    WpAuthenticationProvider::static_with_username_and_password(username, password),
                ))
            }
        }
    }
}

fn env_or_arg(value: &Option<String>, var: &str) -> Option<String> {
    value.clone().or_else(|| std::env::var(var).ok())
}

async fn build_api_client(args: &AuthArgs, url: &Option<String>) -> Result<WpApiClient> {
    let request_executor = Arc::new(ReqwestRequestExecutor::new(false, Duration::from_secs(60)));
    let middleware_pipeline = Arc::new(WpApiMiddlewarePipeline::default());
    // Determine resolver and auth provider
    let api_type = SiteApiType::detect_from_args(args, url, &request_executor).await?;
    let resolver: Arc<dyn ApiUrlResolver> = api_type.api_url_resolver();
    let auth_provider: Arc<WpAuthenticationProvider> = api_type.auth_provider(args)?;

    #[derive(Debug)]
    struct CliAppNotifier;
    #[async_trait::async_trait]
    impl WpAppNotifier for CliAppNotifier {
        async fn requested_with_invalid_authentication(&self, request_url: String) {
            eprintln!(
                "Authentication failed for request to: {}. Please verify your credentials or token and try again.",
                request_url
            );
            std::process::exit(1);
        }
    }

    Ok(WpApiClient::new(
        resolver,
        WpApiClientDelegate {
            auth_provider,
            request_executor,
            middleware_pipeline,
            app_notifier: Arc::new(CliAppNotifier),
        },
    ))
}

fn parse_post_id(s: &str) -> Result<PostId, String> {
    s.parse::<i64>()
        .map(PostId)
        .map_err(|e| format!("Invalid post id '{s}': {e}"))
}

async fn resolve_post_id(client: &WpApiClient, post_url: &str) -> Result<PostId> {
    // Strategy: retrieve by slug via posts list API when possible.
    // For wp.com, the resolver requires site context; for wp.org, api_root is given.
    // We'll try to parse the URL and extract a last path segment as potential slug.
    let url = Url::parse(post_url).map_err(|e| anyhow!("Invalid url: {e}"))?;
    let slug_candidate = url
        .path_segments()
        .and_then(|segs| segs.rev().find(|s| !s.is_empty()))
        .map(|s| s.trim_end_matches('/'))
        .unwrap_or("")
        .to_string();

    if slug_candidate.is_empty() {
        return Err(anyhow!("Could not parse a slug from url"));
    }

    // Query posts by slug; returns an array, take first match.
    // Using view context to ensure public content shape.
    let params = wp_api::posts::PostListParams {
        slug: vec![slug_candidate.clone()],
        per_page: Some(1),
        ..Default::default()
    };
    let resp = client.posts().list_with_view_context(&params).await?;
    if let Some(p) = resp.data.into_iter().map(|sp| sp.id).next() {
        return Ok(p);
    }

    Err(anyhow!(
        "No post found for slug '{slug}' parsed from URL '{url}'",
        slug = slug_candidate,
        url = post_url
    ))
}

async fn fetch_post_and_comments(args: FetchPostArgs) -> Result<()> {
    let client = build_api_client(&args.auth, &args.url).await?;

    let post_id = if let Some(id) = args.post_id {
        id
    } else {
        let post_url = args
            .url
            .as_ref()
            .ok_or_else(|| anyhow!("Either --post-id or --url must be provided"))?;
        resolve_post_id(&client, post_url.as_str()).await?
    };

    let post = client
        .posts()
        .retrieve_with_view_context(
            &post_id,
            &PostRetrieveParams {
                password: args.post_password.clone(),
            },
        )
        .await?;

    let mut all_comments = Vec::new();
    let mut page = client
        .comments()
        .list_with_view_context(&CommentListParams {
            post: vec![post_id],
            per_page: Some(args.per_page),
            ..Default::default()
        })
        .await?;
    all_comments.extend(page.data);
    while let Some(next_params) = page.next_page_params.take() {
        page = client
            .comments()
            .list_with_view_context(&next_params)
            .await?;
        all_comments.extend(page.data);
    }

    let out = serde_json::json!({
        "post": post,
        "comments": all_comments,
    });
    if args.pretty {
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("{}", serde_json::to_string(&out)?);
    }
    Ok(())
}
