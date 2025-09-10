use wp_api::wp_com::{client::WpComApiClient, freshly_pressed::FreshlyPressedListParams};

pub async fn freshly_pressed_test(client: &WpComApiClient) -> anyhow::Result<()> {
    println!("== Freshly Pressed Test ==");
    let params = FreshlyPressedListParams::default();
    client.freshly_pressed().list(&params).await?;
    println!("✅ Get Freshly Pressed");

    Ok(())
}
