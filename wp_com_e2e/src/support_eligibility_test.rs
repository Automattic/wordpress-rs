use wp_api::wp_com::client::WpComApiClient;

pub async fn support_eligibility_test(client: &WpComApiClient) -> anyhow::Result<()> {
    println!("== Support Eligibility Test ==");
    client
        .support_eligibility()
        .get_support_eligibility()
        .await?;
    println!("✅ Get Support Eligibility");

    Ok(())
}
