use wp_api::wp_com::{client::WpComApiClient, oauth2::TokenValidationParameters};

pub async fn me_test(client: &WpComApiClient, token: String) -> anyhow::Result<()> {
    println!("== Current User Info Test ==");

    let user_info = client.me().get().await?.data;
    println!("✅ Get Current User Info");

    if let Some(client_id) = user_info.token_client_id {
        println!("== OAuth 2 Token Test ==");
        client
            .oauth2()
            .fetch_info(&TokenValidationParameters {
                client_id: client_id.to_string(),
                token: token.clone(),
            })
            .await?;
        println!("✅ Get OAuth 2 Token Info");
    }

    Ok(())
}
