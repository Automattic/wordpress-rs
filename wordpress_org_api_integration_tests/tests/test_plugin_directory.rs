use rstest_reuse::*;
use wordpress_org_api::plugin_directory::*;

mod plugin_directory_parameters;
use plugin_directory_parameters::*;

#[apply(query_plugins_api_url)]
#[tokio::test]
async fn test_parse_query_plugins_api(#[case] url: &str) {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await.unwrap();
    let result = response.json::<QueryPluginResponse>().await;

    assert!(
        result.is_ok(),
        "Failed to parse plugin query {:?}: {:?}",
        url,
        result.err()
    );

    let result = result.unwrap();
    assert!(result.plugins.len() > 0);
}

async fn test_parse_plugin_information_api(slug: &str) {
    let url = format!(
        "https://api.wordpress.org/plugins/info/1.2/?action=plugin_information&request[slug]={}&fields=icons",
        slug
    );
    println!("Plugin information API URL: {}", url);

    let client = reqwest::Client::new();
    let response = client.get(url).send().await.unwrap();
    let result = response.json::<PluginInformation>().await;

    assert!(
        result.is_ok(),
        "Failed to parse plugin information {:?}: {:?}",
        slug,
        result.err()
    );

    let result = result.unwrap();
    assert_eq!(result.slug, slug);
}

#[apply(plugin_information_slug_1)]
#[tokio::test]
async fn test_parse_plugin_information_api_1(#[case] slug: &str) {
    test_parse_plugin_information_api(slug).await;
}

#[apply(plugin_information_slug_2)]
#[tokio::test]
async fn test_parse_plugin_information_api_2(#[case] slug: &str) {
    test_parse_plugin_information_api(slug).await;
}
