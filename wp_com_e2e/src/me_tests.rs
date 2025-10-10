use wp_api::wp_com::client::WpComApiClient;

pub async fn me_test(client: &WpComApiClient) -> anyhow::Result<()> {
    println!("== Current User Info Test ==");

    client.me().get().await?;

    println!("✅ Get Current User Info");

    Ok(())
}
