use std::{
    env,
    io::{self, Write},
    sync::Arc,
};

use wp_api::wordpress_org::client::{
    WordPressOrgApiClient, WordPressOrgApiPluginDirectoryCategory,
};

use wp_api_integration_tests::AsyncWpNetworking;

fn wordpress_org_api_client() -> WordPressOrgApiClient {
    WordPressOrgApiClient::new(Arc::new(AsyncWpNetworking::default()))
}

#[tokio::test]
async fn test_parsing_full_plugin_directory() {
    let client = wordpress_org_api_client();

    let page_size: u64;
    let total_pages: u64;
    if env::var("TEST_ALL_PLUGINS").is_ok() {
        println!("Checking how many pages to fetch...");
        page_size = 200;
        let response = client.browse_plugins(None, 1, page_size).await.unwrap();
        total_pages = response.info.pages;
    } else {
        println!("Only a small amount of plugins will be fetched.");
        page_size = 10;
        total_pages = 2;
    }

    let mut query_plugins_failures = Vec::new();
    let mut all_slugs: Vec<String> = Vec::new();
    for page in 1..=total_pages {
        let slugs: Result<Vec<_>, _> = client
            .browse_plugins(None, page, page_size)
            .await
            .map(|r| r.plugins.into_iter().map(|p| p.slug).collect());
        match slugs {
            Ok(slugs) => {
                print!(".");
                all_slugs.extend(slugs);
            }
            Err(e) => {
                print!("F({})", page);
                query_plugins_failures.push((page, e));
            }
        }
        _ = io::stdout().flush();
    }
    println!();

    println!("Fetching and parsing {} plugins...", all_slugs.len());

    let mut plugin_information_failures = Vec::new();
    for slug in all_slugs {
        let info = client.plugin_information(&slug.as_str().into()).await;
        if let Err(e) = info {
            print!("F({})", slug);
            plugin_information_failures.push((slug.to_string(), e));
        } else {
            print!(".");
        }
        _ = io::stdout().flush();
    }
    println!();

    println!("{} query plugins failures:", query_plugins_failures.len());
    for (page, e) in &query_plugins_failures {
        println!("  - Page {:?}, page size {:?} : {:?}", page, page_size, e);
    }

    println!(
        "{} plugin information failures:",
        plugin_information_failures.len()
    );
    for (slug, e) in &plugin_information_failures {
        println!("  - {:?} : {:?}", slug, e);
    }

    assert!(query_plugins_failures.is_empty());
    assert!(plugin_information_failures.is_empty())
}

#[tokio::test]
#[rstest::rstest]
#[case(WordPressOrgApiPluginDirectoryCategory::New)]
#[case(WordPressOrgApiPluginDirectoryCategory::Popular)]
#[case(WordPressOrgApiPluginDirectoryCategory::Updated)]
#[case(WordPressOrgApiPluginDirectoryCategory::TopRated)]
async fn test_browse_plugins(#[case] category: WordPressOrgApiPluginDirectoryCategory) {
    use wp_api_integration_tests::AssertResponse;
    let response = wordpress_org_api_client()
        .browse_plugins(Some(category), 1, 30)
        .await
        .assert_response();
    assert!(!response.plugins.is_empty());
}

#[tokio::test]
async fn test_search_plugins() {
    use wp_api_integration_tests::AssertResponse;
    let plugins = wordpress_org_api_client()
        .search_plugins("jetpack-social".to_string(), 1, 30)
        .await
        .assert_response()
        .plugins;
    assert!(
        plugins.iter().any(|p| p.slug == "jetpack-social"),
        "Plugins search result doesn't contain 'jetpack-social': {:#?}",
        plugins
    );
}
