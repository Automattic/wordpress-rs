//! `wpcom-login` signs in to WordPress.com in a browser and saves the account's bearer token to a file.

use anyhow::{Context, Result, anyhow, bail};
use clap::Args;
use std::{
    collections::hash_map::RandomState,
    hash::{BuildHasher, Hasher},
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};
use url::Url;
use wp_api::{
    prelude::*,
    wp_com::{
        client::WpComApiClient,
        oauth2::{
            TokenRequestParameters, WpComOauthScope, build_token_request_url,
            parse_authorization_url,
        },
    },
};

const CLIENT_ID_ENV: &str = "WP_COM_CLIENT_ID";
const CLIENT_SECRET_ENV: &str = "WP_COM_CLIENT_SECRET";
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, Args)]
pub struct WpComLoginArgs {
    /// JSON file with `client_id` and `client_secret`, such as this repository's
    /// `wp_com_test_credentials.json`. Used for whichever of the two isn't given another way.
    #[arg(long)]
    credentials_file: Option<PathBuf>,

    /// Client ID of a WordPress.com application, from https://developer.wordpress.com/apps/
    /// (fallback env: WP_COM_CLIENT_ID)
    #[arg(long)]
    client_id: Option<u64>,

    /// File holding the application's client secret (fallback env: WP_COM_CLIENT_SECRET).
    /// The secret is never taken as an argument, so it stays out of the shell history.
    #[arg(long)]
    client_secret_file: Option<PathBuf>,

    /// Where WordPress.com sends the browser after sign-in. It must be one of the
    /// application's redirect URLs. A `http://localhost:<port>` address is caught
    /// automatically; for any other address, paste the final address when asked.
    #[arg(long, default_value = "http://localhost:8765/callback")]
    redirect_uri: String,

    /// File to write the token to. It is created readable by you only.
    #[arg(long)]
    out: PathBuf,

    /// Replace `--out` if it already exists.
    #[arg(long)]
    force: bool,

    /// Print the sign-in address instead of opening a browser.
    #[arg(long)]
    no_browser: bool,
}

pub async fn wpcom_login(args: WpComLoginArgs) -> Result<()> {
    if args.out.exists() && !args.force {
        bail!(
            "{} already exists. Pass --force to replace it.",
            args.out.display()
        );
    }
    let client_id = client_id(&args)?;
    let client_secret = client_secret(&args)?;
    let redirect = Url::parse(&args.redirect_uri)
        .with_context(|| format!("--redirect-uri is not a URL: {}", args.redirect_uri))?;

    let state = random_state();
    let sign_in_url = build_token_request_url(
        client_id,
        &args.redirect_uri,
        vec![WpComOauthScope::Global],
        &state,
        None,
    )
    .url();

    // The listener is bound before the browser opens, so the redirect can't arrive first.
    let listener = match local_port(&redirect) {
        Some(port) => Some(
            TcpListener::bind(("127.0.0.1", port))
                .await
                .with_context(|| format!("Port {port} on this machine is already in use."))?,
        ),
        None => None,
    };

    if args.no_browser || !open_in_browser(&sign_in_url) {
        println!("Open this address in a browser and sign in:\n\n{sign_in_url}\n");
    } else {
        println!("A browser window is open. Sign in there with the account you want a token for.");
    }

    let callback = match listener {
        Some(listener) => {
            tokio::time::timeout(SIGN_IN_TIMEOUT, wait_for_callback(listener, &redirect))
                .await
                .map_err(|_| anyhow!("No sign-in arrived within five minutes."))??
        }
        None => read_pasted_callback()?,
    };

    let authorization = parse_authorization_url(callback)
        .map_err(|e| anyhow!("WordPress.com did not approve the sign-in: {e}"))?;
    if authorization.state.as_deref() != Some(state.as_str()) {
        bail!("The `state` in the callback isn't the one this sign-in started with.");
    }

    let token = client(WpAuthenticationProvider::none())
        .oauth2()
        .request_token(&TokenRequestParameters {
            client_id,
            client_secret,
            code: authorization.code,
            grant_type: "authorization_code".to_string(),
            redirect_uri: args.redirect_uri.clone(),
        })
        .await
        .map_err(|e| anyhow!("WordPress.com refused to exchange the code for a token: {e}"))?
        .data
        .access_token;

    write_private_file(&args.out, &token)?;

    let account = client(WpAuthenticationProvider::static_with_auth(
        WpAuthentication::Bearer { token },
    ))
    .me()
    .get()
    .await
    .map_err(|e| anyhow!("The token was saved, but reading its account failed: {e}"))?
    .data;
    println!(
        "Saved a token for {} ({}) to {}.",
        account.username,
        account.display_name,
        args.out.display()
    );

    Ok(())
}

#[derive(serde::Deserialize)]
struct CredentialsFile {
    #[serde(default)]
    client_id: u64,
    #[serde(default)]
    client_secret: String,
}

fn credentials_file(args: &WpComLoginArgs) -> Result<Option<CredentialsFile>> {
    let Some(path) = &args.credentials_file else {
        return Ok(None);
    };
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Could not read {}.", path.display()))?;
    // The parse error is left out: it can quote the line it failed on, which may hold the secret.
    let credentials = serde_json::from_str(&contents)
        .map_err(|_| anyhow!("{} is not a JSON credentials file.", path.display()))?;
    Ok(Some(credentials))
}

fn client_id(args: &WpComLoginArgs) -> Result<u64> {
    if let Some(client_id) = args.client_id {
        return Ok(client_id);
    }
    if let Ok(client_id) = std::env::var(CLIENT_ID_ENV) {
        return client_id
            .trim()
            .parse()
            .with_context(|| format!("{CLIENT_ID_ENV} is not a number."));
    }
    match credentials_file(args)? {
        Some(credentials) if credentials.client_id != 0 => Ok(credentials.client_id),
        Some(_) => bail!("The credentials file has no `client_id`."),
        None => bail!("Pass --client-id or --credentials-file, or set {CLIENT_ID_ENV}."),
    }
}

fn client_secret(args: &WpComLoginArgs) -> Result<String> {
    let secret = if let Some(path) = &args.client_secret_file {
        std::fs::read_to_string(path)
            .with_context(|| format!("Could not read {}.", path.display()))?
    } else if let Ok(secret) = std::env::var(CLIENT_SECRET_ENV) {
        secret
    } else if let Some(credentials) = credentials_file(args)? {
        credentials.client_secret
    } else {
        bail!("Pass --client-secret-file or --credentials-file, or set {CLIENT_SECRET_ENV}.");
    };
    let secret = secret.trim().to_string();
    if secret.is_empty() {
        bail!("The client secret is empty.");
    }
    Ok(secret)
}

/// Returns the port to listen on when `redirect` points at this machine over plain HTTP.
fn local_port(redirect: &Url) -> Option<u16> {
    let is_local = matches!(redirect.host_str(), Some("localhost" | "127.0.0.1"));
    (redirect.scheme() == "http" && is_local).then(|| redirect.port().unwrap_or(80))
}

/// Returns a value that WordPress.com echoes back, so a callback from another sign-in is rejected.
fn random_state() -> String {
    let word = || RandomState::new().build_hasher().finish();
    format!("{:016x}{:016x}", word(), word())
}

fn open_in_browser(url: &str) -> bool {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        return false;
    } else {
        "xdg-open"
    };
    std::process::Command::new(program)
        .arg(url)
        .status()
        .is_ok_and(|status| status.success())
}

/// Waits for the browser to request the redirect address, and returns the full address it asked for.
async fn wait_for_callback(listener: TcpListener, redirect: &Url) -> Result<String> {
    loop {
        let (stream, _) = listener.accept().await?;
        let mut stream = BufReader::new(stream);
        let mut request_line = String::new();
        stream.read_line(&mut request_line).await?;

        // "GET /callback?code=…&state=… HTTP/1.1". Browsers also ask for things like /favicon.ico.
        let target = request_line.split_whitespace().nth(1).unwrap_or_default();
        let is_callback = target.split('?').next() == Some(redirect.path()) && target.contains('?');
        let body = if is_callback {
            "Signed in. You can close this window and go back to the terminal."
        } else {
            "Nothing here."
        };
        let status = if is_callback {
            "200 OK"
        } else {
            "404 Not Found"
        };
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        // The browser may already be gone. The address is what matters.
        let _ = stream.get_mut().write_all(response.as_bytes()).await;
        let _ = stream.get_mut().shutdown().await;

        if is_callback {
            let mut origin = redirect.clone();
            origin.set_path("");
            origin.set_query(None);
            return Ok(format!("{}{target}", origin.as_str().trim_end_matches('/')));
        }
    }
}

fn read_pasted_callback() -> Result<String> {
    print!("After signing in, paste the address the browser was sent to, then press Return:\n> ");
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    let line = line.trim();
    if line.is_empty() {
        bail!("No address was pasted.");
    }
    Ok(line.to_string())
}

fn write_private_file(path: &Path, contents: &str) -> Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .with_context(|| format!("Could not write {}.", path.display()))?;
    // An existing file keeps its old permissions when it's reopened, so they're set again.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    writeln!(file, "{contents}")?;
    Ok(())
}

fn client(auth_provider: WpAuthenticationProvider) -> WpComApiClient {
    #[derive(Debug)]
    struct QuietAppNotifier;
    #[async_trait::async_trait]
    impl WpAppNotifier for QuietAppNotifier {
        async fn requested_with_invalid_authentication(&self, _request_url: String) {}
    }

    WpComApiClient::new(WpApiClientDelegate {
        auth_provider: Arc::new(auth_provider),
        request_executor: Arc::new(ReqwestRequestExecutor::new(false, Duration::from_secs(60))),
        middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
        app_notifier: Arc::new(QuietAppNotifier),
        language_provider: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_port_is_found_only_for_plain_http_on_this_machine() {
        let port = |url: &str| local_port(&Url::parse(url).unwrap());
        assert_eq!(port("http://localhost:8765/callback"), Some(8765));
        assert_eq!(port("http://127.0.0.1:9000/"), Some(9000));
        assert_eq!(port("http://localhost/callback"), Some(80));
        assert_eq!(port("https://localhost:8765/callback"), None);
        assert_eq!(port("x-wordpress-app://oauth2-callback"), None);
        assert_eq!(port("http://example.com:8765/callback"), None);
    }

    fn args(credentials_file: Option<PathBuf>) -> WpComLoginArgs {
        WpComLoginArgs {
            credentials_file,
            client_id: None,
            client_secret_file: None,
            redirect_uri: "http://localhost:8765/callback".to_string(),
            out: PathBuf::from("unused"),
            force: false,
            no_browser: true,
        }
    }

    #[test]
    fn client_id_and_secret_are_read_from_a_credentials_file() {
        let path = std::env::temp_dir().join(format!("wpcom-login-test-{}.json", random_state()));
        std::fs::write(
            &path,
            r#"{"bearer_token": "ignored", "site_id": 0, "client_id": 42, "client_secret": " shh \n"}"#,
        )
        .unwrap();
        let args = args(Some(path.clone()));

        assert_eq!(client_id(&args).unwrap(), 42);
        assert_eq!(client_secret(&args).unwrap(), "shh");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_credentials_file_without_values_is_an_error_that_does_not_quote_it() {
        let path = std::env::temp_dir().join(format!("wpcom-login-test-{}.json", random_state()));
        std::fs::write(&path, r#"{"client_id": 0, "client_secret": ""}"#).unwrap();
        let with_empty_values = args(Some(path.clone()));
        assert!(client_id(&with_empty_values).is_err());
        assert!(client_secret(&with_empty_values).is_err());

        std::fs::write(&path, r#"{"client_secret": "hunter2" oops"#).unwrap();
        let broken = client_secret(&args(Some(path.clone())))
            .unwrap_err()
            .to_string();
        assert!(!broken.contains("hunter2"), "{broken}");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn random_state_differs_between_calls() {
        assert_ne!(random_state(), random_state());
        assert_eq!(random_state().len(), 32);
    }

    #[tokio::test]
    async fn wait_for_callback_ignores_other_requests_and_returns_the_full_address() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let redirect = Url::parse(&format!("http://localhost:{port}/callback")).unwrap();

        let browser = tokio::spawn(async move {
            for target in ["/favicon.ico", "/callback?code=abc&state=xyz"] {
                let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
                    .await
                    .unwrap();
                let request = format!("GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n");
                stream.write_all(request.as_bytes()).await.unwrap();
                let mut response = String::new();
                tokio::io::AsyncReadExt::read_to_string(&mut stream, &mut response)
                    .await
                    .unwrap();
            }
        });

        let callback = wait_for_callback(listener, &redirect).await.unwrap();
        browser.await.unwrap();

        assert_eq!(
            callback,
            format!("http://localhost:{port}/callback?code=abc&state=xyz")
        );
        let parsed = parse_authorization_url(callback).unwrap();
        assert_eq!(parsed.code, "abc");
        assert_eq!(parsed.state.as_deref(), Some("xyz"));
    }

    #[cfg(unix)]
    #[test]
    fn write_private_file_is_readable_by_the_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir().join(format!("wpcom-login-test-{}", random_state()));
        std::fs::write(&path, "old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        write_private_file(&path, "token").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "token\n");
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        std::fs::remove_file(&path).unwrap();
    }
}
