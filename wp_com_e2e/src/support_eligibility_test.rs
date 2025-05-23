use anyhow::{Ok, Result};
use async_trait::async_trait;
use wp_api::wp_com::client::WpComApiClient;

use crate::Testable;

pub struct SupportEligibilityTest<'a> {
    pub client: &'a WpComApiClient,
}

#[async_trait]
impl Testable for SupportEligibilityTest<'_> {
    async fn test(&self) -> Result<(), anyhow::Error> {
        println!("== Support Eligibility Test ==");
        self.client
            .support_eligibility()
            .get_support_eligibility()
            .await?;
        println!("✅ Get Support Eligibility");

        Ok(())
    }
}
