use serde::{Serialize, de::DeserializeOwned};
use wp_api::{
    categories::CategoryId, comments::CommentId, posts::PostId, tags::TagId, users::UserId,
};
use wp_cli::{
    WpCliCategory, WpCliComment, WpCliPost, WpCliSiteSettings, WpCliTag, WpCliUser, WpCliUserMeta,
};

const BACKEND_ADDRESS: &str = "http://127.0.0.1:4000";
const BACKEND_PATH_RESTORE: &str = "/restore";
const BACKEND_PATH_CATEGORY: &str = "/wp-cli/category";
const BACKEND_PATH_CATEGORIES: &str = "/wp-cli/categories";
const BACKEND_PATH_COMMENT: &str = "/wp-cli/comment";
const BACKEND_PATH_COMMENTS: &str = "/wp-cli/comments";
const BACKEND_PATH_SITE_SETTINGS: &str = "/wp-cli/site-settings";
const BACKEND_PATH_POST: &str = "/wp-cli/post";
const BACKEND_PATH_POSTS: &str = "/wp-cli/posts";
const BACKEND_PATH_TAG: &str = "/wp-cli/tag";
const BACKEND_PATH_TAGS: &str = "/wp-cli/tags";
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
    pub async fn category(category_id: &CategoryId) -> WpCliCategory {
        Self::get(format!("{BACKEND_PATH_CATEGORY}?category_id={category_id}"))
            .await
            .expect("Failed to parse fetched category from wp_cli")
    }
    pub async fn categories() -> Vec<WpCliCategory> {
        Self::get(BACKEND_PATH_CATEGORIES)
            .await
            .expect("Failed to parse fetched categories from wp_cli")
    }
    pub async fn comment(comment_id: &CommentId) -> WpCliComment {
        Self::get(format!("{BACKEND_PATH_COMMENT}?comment_id={comment_id}"))
            .await
            .expect("Failed to parse fetched comment from wp_cli")
    }
    pub async fn comments(comment_status: Option<&str>) -> Vec<WpCliComment> {
        let url = if let Some(comment_status) = comment_status {
            format!("{BACKEND_PATH_COMMENTS}?comment_status={comment_status}")
        } else {
            BACKEND_PATH_COMMENTS.to_string()
        };
        Self::get(url)
            .await
            .expect("Failed to parse fetched comments from wp_cli")
    }
    pub async fn site_settings() -> Result<WpCliSiteSettings, reqwest::Error> {
        Self::get(BACKEND_PATH_SITE_SETTINGS).await
    }
    pub async fn post(post_id: &PostId) -> WpCliPost {
        Self::get(format!("{BACKEND_PATH_POST}?post_id={post_id}"))
            .await
            .expect("Failed to parse fetched post from wp_cli")
    }
    pub async fn posts(post_status: Option<&str>) -> Vec<WpCliPost> {
        let url = if let Some(post_status) = post_status {
            format!("{BACKEND_PATH_POSTS}?post_status={post_status}")
        } else {
            BACKEND_PATH_POSTS.to_string()
        };
        Self::get(url)
            .await
            .expect("Failed to parse fetched posts from wp_cli")
    }
    pub async fn tag(tag_id: &TagId) -> WpCliTag {
        Self::get(format!("{BACKEND_PATH_TAG}?tag_id={tag_id}"))
            .await
            .expect("Failed to parse fetched tag from wp_cli")
    }
    pub async fn tags() -> Vec<WpCliTag> {
        Self::get(BACKEND_PATH_TAGS)
            .await
            .expect("Failed to parse fetched tags from wp_cli")
    }
    pub async fn user(user_id: &UserId) -> WpCliUser {
        Self::get(format!("{BACKEND_PATH_USER}?user_id={user_id}"))
            .await
            .expect("Failed to parse fetched user from wp_cli")
    }
    pub async fn users() -> Vec<WpCliUser> {
        Self::get(BACKEND_PATH_USERS)
            .await
            .expect("Failed to parse fetched users from wp_cli")
    }
    pub async fn user_meta(user_id: &UserId) -> Vec<WpCliUserMeta> {
        Self::get(format!("{BACKEND_PATH_USER_META}?user_id={user_id}"))
            .await
            .expect("Failed to parse fetched user meta from wp_cli")
    }
    async fn restore(db: bool, plugins: bool) {
        let url = format!("{BACKEND_ADDRESS}{BACKEND_PATH_RESTORE}?db={db}&plugins={plugins}");
        reqwest::get(url)
            .await
            .unwrap_or_else(|_| panic!("Restoring server failed: (db({db}), plugins({plugins}))"));
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
