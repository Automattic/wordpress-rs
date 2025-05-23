use anyhow::{Ok, Result};
use async_trait::async_trait;
use wp_api::wp_com::{client::WpComApiClient, oauth2::TokenValidationParameters};

use crate::Testable;

pub struct Oauth2Test<'a> {
    pub token: &'a String,
    pub client: &'a WpComApiClient,
}

#[async_trait]
impl Testable for Oauth2Test<'_> {
    async fn test(&self) -> Result<(), anyhow::Error> {
        println!("== OAuth 2 Token Test ==");
        self.client
            .oauth2()
            .fetch_info(&TokenValidationParameters {
                client_id: "11".to_string(),
                token: self.token.clone(),
            })
            .await?;
        println!("✅ Get OAuth 2 Token Info");

        Ok(())
    }
}
