use futures::{FutureExt, StreamExt};
use std::{env, sync::Arc, time::Duration};
use wp_api::{
    middleware::WpApiMiddlewarePipeline,
    reqwest_request_executor::ReqwestRequestExecutor,
    wordpress_org::{
        client::{
            WordPressOrgApiClient, WordPressOrgApiClientError,
            WordPressOrgApiPluginDirectoryCategory,
        },
        plugin_directory::PluginInformation,
    },
};

const FETCH_PLUGIN_INFORMATION_RETRY_COUNT: usize = 5;
const TOKIO_STREAM_SIZE: usize = 100;

fn wordpress_org_api_client() -> WordPressOrgApiClient {
    WordPressOrgApiClient::new(
        Arc::new(ReqwestRequestExecutor::new(false, Duration::from_secs(120))),
        Arc::new(WpApiMiddlewarePipeline::default()),
    )
}

#[tokio::test]
async fn test_parsing_full_plugin_directory() {
    let client = wordpress_org_api_client();

    let (all_slugs, all_slugs_were_fetched_successfully) = query_all_plugin_slugs(&client).await;
    let all_plugin_infos_were_fetched_and_parsed_successfully =
        fetch_plugin_information_for_all_slugs(&client, all_slugs).await;

    assert!(all_slugs_were_fetched_successfully);
    assert!(all_plugin_infos_were_fetched_and_parsed_successfully);
}

async fn query_all_plugin_slugs(client: &WordPressOrgApiClient) -> (Vec<String>, bool) {
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

    println!("Pages to fetch: {total_pages}");

    let results = futures::future::join_all((1..=total_pages).map(|page| {
        client
            .browse_plugins(None, page, page_size)
            .map(move |r| (page, r))
    }))
    .await;

    let mut query_plugins_failures = Vec::new();
    let mut all_slugs: Vec<String> = Vec::new();

    for (page, query_plugins_response) in results {
        match query_plugins_response {
            Ok(r) => all_slugs.extend(r.plugins.into_iter().map(|p| p.slug.slug)),
            Err(e) => query_plugins_failures.push((page, e)),
        }
    }

    if !query_plugins_failures.is_empty() {
        println!(
            "Number of failures while querying plugin slugs: {}",
            query_plugins_failures.len()
        );
        for (page, e) in &query_plugins_failures {
            println!("  - Page {:?}, page size {:?} : {:?}", page, page_size, e);
        }
    } else {
        println!("Successfully fetched {total_pages} query plugins pages!");
    }

    (all_slugs, query_plugins_failures.is_empty())
}

async fn fetch_plugin_information_for_all_slugs(
    client: &WordPressOrgApiClient,
    all_slugs: Vec<String>,
) -> bool {
    let number_of_plugins = all_slugs.len();
    println!("Fetching and parsing {number_of_plugins} plugins...");

    let mut plugin_info_stream = tokio_stream::iter(all_slugs)
        .map(|slug| fetch_plugin_information(client, slug, FETCH_PLUGIN_INFORMATION_RETRY_COUNT))
        .buffer_unordered(TOKIO_STREAM_SIZE);

    let mut plugin_information_failures = Vec::new();
    while let Some((slug, info_result)) = plugin_info_stream.next().await {
        if let Err(e) = info_result {
            plugin_information_failures.push((slug, e));
        }
    }

    if !plugin_information_failures.is_empty() {
        println!(
            "{} plugin information failures:",
            plugin_information_failures.len()
        );
        for (slug, e) in &plugin_information_failures {
            println!("  - {:?} : {:?}", slug, e);
        }
    } else {
        println!("Successfully fetched and parsed {number_of_plugins} plugins!");
    }

    plugin_information_failures.is_empty()
}

async fn fetch_plugin_information(
    client: &WordPressOrgApiClient,
    slug: String,
    remaining_retry_count: usize,
) -> (
    String,
    Result<PluginInformation, WordPressOrgApiClientError>,
) {
    let result = client.plugin_information(&slug.as_str().into()).await;
    if remaining_retry_count == 0 {
        (slug, result)
    } else if let Err(WordPressOrgApiClientError::RequestExecutionFailed { .. }) = result {
        println!(
            "Retry fetching '{slug}', remaining retries: {}",
            remaining_retry_count - 1
        );
        Box::pin(fetch_plugin_information(
            client,
            slug.clone(),
            remaining_retry_count - 1,
        ))
        .await
    } else {
        (slug, result)
    }
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
        plugins.iter().any(|p| p.slug == "jetpack-social".into()),
        "Plugins search result doesn't contain 'jetpack-social': {:#?}",
        plugins
    );
}
