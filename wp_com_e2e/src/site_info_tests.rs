use wp_api::wp_com::{client::WpComApiClient, site_info::SiteInfoParameters};

pub async fn site_info_test(client: &WpComApiClient) -> anyhow::Result<()> {
    println!("== Site Info Test ==");

    println!("Testing developer.wordpress.com...");
    let response = client
        .site_info()
        .fetch(&SiteInfoParameters {
            url: "https://developer.wordpress.com".to_string(),
        })
        .await?
        .data;
    println!("  URL after redirects: {}", response.url_after_redirects);
    println!("  Is WordPress.com: {}", response.is_word_press_dot_com);
    println!("  Is WordPress: {}", response.is_word_press);
    assert!(
        response.is_word_press_dot_com,
        "developer.wordpress.com should be a WordPress.com site"
    );
    assert!(
        response.is_word_press,
        "developer.wordpress.com should be a WordPress site"
    );
    println!("✅ developer.wordpress.com verified as WordPress.com site");

    println!("\nTesting developer.wordpress.org...");
    let response = client
        .site_info()
        .fetch(&SiteInfoParameters {
            url: "https://developer.wordpress.org".to_string(),
        })
        .await?
        .data;
    println!("  URL after redirects: {}", response.url_after_redirects);
    println!("  Is WordPress.com: {}", response.is_word_press_dot_com);
    println!("  Is WordPress: {}", response.is_word_press);
    println!("  Has Jetpack: {}", response.has_jetpack);
    assert!(
        response.is_word_press,
        "developer.wordpress.org should be a WordPress site"
    );
    println!("✅ developer.wordpress.org verified as WordPress site");

    println!("\nTesting google.com...");
    let response = client
        .site_info()
        .fetch(&SiteInfoParameters {
            url: "https://google.com".to_string(),
        })
        .await?
        .data;
    println!("  URL after redirects: {}", response.url_after_redirects);
    println!("  Is WordPress.com: {}", response.is_word_press_dot_com);
    println!("  Is WordPress: {}", response.is_word_press);
    assert!(
        !response.is_word_press,
        "google.com should not be a WordPress site"
    );
    assert!(
        !response.is_word_press_dot_com,
        "google.com should not be a WordPress.com site"
    );
    println!("✅ google.com verified as non-WordPress site");

    println!("\n✅ All Site Info Tests Passed");
    Ok(())
}
