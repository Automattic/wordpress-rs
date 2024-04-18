use std::process::Command;

use futures::Future;
use sqlx::{mysql::MySqlConnectOptions, ConnectOptions, MySqlConnection};

pub async fn run_and_restore<F, Fut>(f: F)
where
    F: FnOnce(WordPressDb) -> Fut,
    Fut: Future<Output = ()>,
{
    let options = MySqlConnectOptions::new()
        .host("localhost")
        .username("wordpress")
        .password("wordpress")
        .database("wordpress");
    let conn = MySqlConnectOptions::connect(&options).await.unwrap();
    f(WordPressDb { conn }).await
}

pub struct WordPressDb {
    conn: MySqlConnection,
}

impl Drop for WordPressDb {
    fn drop(&mut self) {
        println!("Restoring WordPressDB..");
        Command::new("make")
            .arg("-C")
            .arg("../")
            .arg("restore-mysql")
            .status()
            .expect("Failed to restore db");
    }
}

#[allow(dead_code)]
impl WordPressDb {
    pub async fn fetch_db_users(&mut self) -> Result<Vec<DbUser>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM wp_users")
            .fetch_all(&mut self.conn)
            .await
    }

    pub async fn fetch_db_user(&mut self, user_id: u64) -> Result<DbUser, sqlx::Error> {
        sqlx::query_as("SELECT * FROM wp_users WHERE ID = ?")
            .bind(user_id)
            .fetch_one(&mut self.conn)
            .await
    }
}

#[allow(dead_code)]
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
