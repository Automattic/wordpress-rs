use serde::{de::DeserializeOwned, Serialize};
use wp_api::{posts::PostId, users::UserId};
use wp_cli::{WpCliPost, WpCliSiteSettings, WpCliUser, WpCliUserMeta};

const BACKEND_ADDRESS: &str = "http://127.0.0.1:4000";
const BACKEND_PATH_RESTORE: &str = "/restore";
const BACKEND_PATH_SITE_SETTINGS: &str = "/wp-cli/site-settings";
const BACKEND_PATH_POST: &str = "/wp-cli/post";
const BACKEND_PATH_POSTS: &str = "/wp-cli/posts";
const BACKEND_PATH_USER: &str = "/wp-cli/user";
const BACKEND_PATH_USERS: &str = "/wp-cli/users";
const BACKEND_PATH_USER_META: &str = "/wp-cli/user-meta";

#[derive(Debug)]
pub struct Backend;

impl Backend {
    async fn get<T: DeserializeOwned>(path: impl AsRef<str>) -> Result<T, reqwest::Error> {
        let url = format!("{}{}", BACKEND_ADDRESS, path.as_ref());
        reqwest::get(url).await?.json().await
    }
    pub async fn site_settings() -> Result<WpCliSiteSettings, reqwest::Error> {
        Self::get(BACKEND_PATH_SITE_SETTINGS).await
    }
    pub async fn post(post_id: &PostId) -> WpCliPost {
        Self::get(format!("{}?post_id={}", BACKEND_PATH_POST, post_id))
            .await
            .expect("Failed to parse fetched post from wp_cli")
    }
    pub async fn posts(post_status: Option<&str>) -> Vec<WpCliPost> {
        let url = if let Some(post_status) = post_status {
            format!("{}?post_status={}", BACKEND_PATH_POSTS, post_status)
        } else {
            BACKEND_PATH_POSTS.to_string()
        };
        Self::get(url)
            .await
            .expect("Failed to parse fetched posts from wp_cli")
    }
    pub async fn user(user_id: &UserId) -> WpCliUser {
        Self::get(format!("{}?user_id={}", BACKEND_PATH_USER, user_id))
            .await
            .expect("Failed to parse fetched user from wp_cli")
    }
    pub async fn users() -> Vec<WpCliUser> {
        Self::get(BACKEND_PATH_USERS)
            .await
            .expect("Failed to parse fetched users from wp_cli")
    }
    pub async fn user_meta(user_id: &UserId) -> Vec<WpCliUserMeta> {
        Self::get(format!("{}?user_id={}", BACKEND_PATH_USER_META, user_id))
            .await
            .expect("Failed to parse fetched user meta from wp_cli")
    }
    async fn restore(db: bool, plugins: bool) {
        let url = format!(
            "{}{}?db={}&plugins={}",
            BACKEND_ADDRESS, BACKEND_PATH_RESTORE, db, plugins
        );
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
