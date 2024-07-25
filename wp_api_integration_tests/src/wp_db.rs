use futures::Future;
use sqlx::types::chrono;
use sqlx::Executor;
use sqlx::{mysql::MySqlConnectOptions, ConnectOptions, MySqlConnection};

pub async fn run_and_restore<F, Fut>(f: F)
where
    F: FnOnce(WordPressDb) -> Fut,
    Fut: Future<Output = ()>,
{
    let hostname = std::env::var("DB_HOSTNAME").unwrap_or("host.docker.internal".to_string());
    let wp_content_path =
        std::env::var("WP_CONTENT_PATH").unwrap_or("/app/.wordpress/wp-content".to_string());
    let db_dump_path = wp_content_path + "/dump.sql";

    let options = MySqlConnectOptions::new()
        .host(hostname.as_str())
        .username("wordpress")
        .password("wordpress")
        .database("wordpress");
    let conn = MySqlConnectOptions::connect(&options).await.unwrap();
    let db = WordPressDb { conn };

    let result = f(db).await;

    let mut cleanup_conn = MySqlConnectOptions::connect(&options).await.unwrap();
    println!("Restoring WordPressDB from {:?}", db_dump_path);

    let db_schema = std::fs::read_to_string(db_dump_path).expect("Failed to read SQL dump");

    let _ = &mut cleanup_conn
        .execute(db_schema.as_str())
        .await
        .expect("Failed to restore database");

    result
}

#[derive(Debug)]
pub struct WordPressDb {
    conn: MySqlConnection,
}

impl WordPressDb {
    pub async fn users(&mut self) -> Result<Vec<DbUser>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM wp_users")
            .fetch_all(&mut self.conn)
            .await
    }

    pub async fn user(&mut self, user_id: u64) -> Result<DbUser, sqlx::Error> {
        sqlx::query_as("SELECT * FROM wp_users WHERE ID = ?")
            .bind(user_id)
            .fetch_one(&mut self.conn)
            .await
    }

    pub async fn user_meta(&mut self, user_id: u64) -> Result<Vec<DbUserMeta>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM wp_usermeta WHERE user_id = ?")
            .bind(user_id)
            .fetch_all(&mut self.conn)
            .await
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct DbUser {
    #[sqlx(rename = "ID")]
    pub id: u64,
    #[sqlx(rename = "user_login")]
    pub username: String,
    #[sqlx(rename = "user_nicename")]
    pub slug: String,
    #[sqlx(rename = "user_email")]
    pub email: String,
    #[sqlx(rename = "user_url")]
    pub url: String,
    #[sqlx(rename = "user_registered")]
    pub registered_date: chrono::DateTime<chrono::Utc>,
    #[sqlx(rename = "display_name")]
    pub name: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct DbUserMeta {
    pub user_id: u64,
    pub meta_key: String,
    pub meta_value: String,
}
