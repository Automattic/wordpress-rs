use serde::Deserialize;
use wordpress_org_api::plugin_directory::*;

async fn query_plugins_slugs(url: &str) -> Result<Vec<String>, reqwest::Error> {
    #[derive(Deserialize, Debug)]
    struct Response {
        plugins: Vec<Plugin>,
    }
    #[derive(Deserialize, Debug)]
    struct Plugin {
        slug: String,
    }

    reqwest::get(url)
        .await?
        .json::<Response>()
        .await?
        .plugins
        .iter()
        .map(|p| Ok(p.slug.clone()))
        .collect()
}

async fn plugin_information(slug: &str) -> Result<PluginInformation, reqwest::Error> {
    let url = format!(
        "https://api.wordpress.org/plugins/info/1.2/?action=plugin_information&request[slug]={}&fields=icons",
        slug
    );
    reqwest::get(&url)
        .await?
        .json::<PluginInformation>()
        .await
        .map_err(Into::into)
}

#[tokio::test]
async fn test_parsing_full_plugin_directory() {
    println!("Checking how many pages to fetch...");
    let page_size = 200;
    let url = format!(
        "https://api.wordpress.org/plugins/info/1.2/?action=query_plugins&request[per_page]={}",
        page_size
    );
    let response = reqwest::get(&url).await.unwrap();
    let json = response.json::<serde_json::Value>().await.unwrap();
    let total_pages = json["info"]["pages"].as_u64().unwrap();
    println!("Total pages: {}", total_pages);

    let mut query_plugins_failures = Vec::new();
    let mut all_slugs: Vec<String> = Vec::new();
    for page in 1..=total_pages {
        println!("Processing page {}...", page);

        let url = format!(
            "https://api.wordpress.org/plugins/info/1.2/?action=query_plugins&request[per_page]={}&request[page]={}",
            page_size, page
        );
        let slugs = query_plugins_slugs(&url).await;
        match slugs {
            Ok(slugs) => {
                all_slugs.extend(slugs);
            }
            Err(e) => {
                println!("Failed to fetch page: {:?}", url);
                query_plugins_failures.push((url, e));
            }
        }
    }

    println!("Fetching and parsing {} plugins...", all_slugs.len());

    let mut plugin_information_failures = Vec::new();
    for slug in all_slugs {
        println!("Fetching plugin information for: {}", slug);
        let info = plugin_information(&slug).await;
        if let Err(e) = info {
            println!("Failed to fetch plugin information: {:?}", slug);
            plugin_information_failures.push((slug.to_string(), e));
        }
    }

    println!("{} query plugins failures:", query_plugins_failures.len());
    for (url, e) in query_plugins_failures {
        println!("  - {:?} : {:?}", url, e);
    }

    println!(
        "{} plugin information failures:",
        plugin_information_failures.len()
    );
    for (slug, e) in plugin_information_failures {
        println!("  - {:?} : {:?}", slug, e);
    }
}
