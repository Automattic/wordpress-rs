use serde::Serialize;
use url::Url;
use wp_api::users::UserId;
use wp_cli::{WpCliSiteSettings, WpCliUser, WpCliUserMeta};

const BACKEND_ADDRESS: &str = "http://127.0.0.1:4000";
const BACKEND_PATH_RESTORE: &str = "/restore";
const BACKEND_PATH_SITE_SETTINGS: &str = "/wp-cli/site-settings";
const BACKEND_PATH_USER: &str = "/wp-cli/user";
const BACKEND_PATH_USERS: &str = "/wp-cli/users";
const BACKEND_PATH_USER_META: &str = "/wp-cli/user-meta";

#[derive(Debug)]
pub struct Backend {
    client: reqwest::Client,
}

impl Default for Backend {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Backend {
    pub async fn site_settings() -> Result<WpCliSiteSettings, reqwest::Error> {
        Self::default()
            .client
            .get(format!("{}{}", BACKEND_ADDRESS, BACKEND_PATH_SITE_SETTINGS))
            .send()
            .await?
            .json()
            .await
    }
    pub async fn user(user_id: &UserId) -> WpCliUser {
        Self::default()
            .client
            .get(format!(
                "{}{}?user_id={}",
                BACKEND_ADDRESS, BACKEND_PATH_USER, user_id
            ))
            .send()
            .await
            .expect("Failed to fetch user from wp_cli")
            .json()
            .await
            .expect("Failed to parse fetched user from wp_cli")
    }
    pub async fn users() -> Vec<WpCliUser> {
        Self::default()
            .client
            .get(format!("{}{}", BACKEND_ADDRESS, BACKEND_PATH_USERS))
            .send()
            .await
            .expect("Failed to fetch users from wp_cli")
            .json()
            .await
            .expect("Failed to parse fetched users from wp_cli")
    }
    pub async fn user_meta(user_id: &UserId) -> Vec<WpCliUserMeta> {
        Self::default()
            .client
            .get(format!(
                "{}{}?user_id={}",
                BACKEND_ADDRESS, BACKEND_PATH_USER_META, user_id
            ))
            .send()
            .await
            .expect("Failed to fetch user meta from wp_cli")
            .json()
            .await
            .expect("Failed to parse fetched user meta from wp_cli")
    }

    async fn restore(db: bool, plugins: bool) {
        let mut url = Url::parse(BACKEND_ADDRESS)
            .expect("BACKEND_ADDRESS is a valid URL")
            .join(BACKEND_PATH_RESTORE)
            .expect("BACKEND_PATH_RESTORE is a valid path");
        url.query_pairs_mut()
            .append_pair("db", db.to_string().as_str())
            .append_pair("plugins", plugins.to_string().as_str());
        reqwest::get(url).await.unwrap_or_else(|_| {
            panic!(
                "Restoring server failed: (db({}), plugins({}))",
                db, plugins
            )
        });
    }
}

#[derive(Debug, Serialize)]
pub struct RestoreServer;

impl RestoreServer {
    pub async fn db() {
        Backend::restore(true, false).await;
    }

    pub async fn all() {
        Backend::restore(true, true).await;
    }
}
