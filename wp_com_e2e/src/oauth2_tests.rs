use wp_api::wp_com::{client::WpComApiClient, oauth2::TokenValidationParameters};

pub async fn oauth2_test(client: &WpComApiClient, token: String) -> anyhow::Result<()> {
    println!("== OAuth 2 Token Test ==");
    client
        .oauth2()
        .fetch_info(&TokenValidationParameters {
            client_id: "11".to_string(),
            token,
        })
        .await?;
    println!("✅ Get OAuth 2 Token Info");

    Ok(())
}
