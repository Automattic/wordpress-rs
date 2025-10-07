use wp_api::wp_com::{client::WpComApiClient, sites::SitesListParams};

pub async fn sites_test(client: &WpComApiClient) -> anyhow::Result<()> {
    println!("== Sites Test ==");

    let sites = client
        .sites()
        .get(&SitesListParams::default())
        .await?
        .data
        .sites;

    println!("✅ Get Sites: Found {} sites", sites.len());

    for site in sites.as_slice() {
        if let Err(e) = client.sites().get_site_by_id(&site.id).await {
            println!("❌ Get Site by ID: {} Error: {}", site.id, e);
            return Err(e.into());
        } else {
            println!("✅ Get Site by ID: {}", site.id);
        }
    }

    for site in sites.as_slice() {
        if let Err(e) = client
            .sites()
            .get_site_by_handle(site.slug.as_ref().unwrap())
            .await
        {
            println!(
                "❌ Get Site by Handle: {} Error: {}",
                site.slug.as_ref().unwrap(),
                e
            );
            return Err(e.into());
        } else {
            println!("✅ Get Site by Handle: {}", site.slug.as_ref().unwrap());
        }
    }

    Ok(())
}
