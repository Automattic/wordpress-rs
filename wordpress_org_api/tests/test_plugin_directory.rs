use std::{
    env,
    io::{self, Write},
};

use serde::Deserialize;
use wordpress_org_api::Client;

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

#[tokio::test]
async fn test_parsing_full_plugin_directory() {
    let page_size: u64;
    let total_pages: u64;
    if env::var("TEST_ALL_PLUGINS").is_ok() {
        println!("Checking how many pages to fetch...");
        page_size = 200;
        let url = format!(
            "https://api.wordpress.org/plugins/info/1.2/?action=query_plugins&request[per_page]={}",
            page_size
        );
        let response = reqwest::get(&url).await.unwrap();
        let json = response.json::<serde_json::Value>().await.unwrap();
        total_pages = json["info"]["pages"].as_u64().unwrap();
    } else {
        println!("Only a small amount of plugins will be fetched.");
        page_size = 10;
        total_pages = 2;
    }

    let mut query_plugins_failures = Vec::new();
    let mut all_slugs: Vec<String> = Vec::new();
    for page in 1..=total_pages {
        let url = format!(
            "https://api.wordpress.org/plugins/info/1.2/?action=query_plugins&request[per_page]={}&request[page]={}",
            page_size, page
        );
        let slugs = query_plugins_slugs(&url).await;
        match slugs {
            Ok(slugs) => {
                print!(".");
                all_slugs.extend(slugs);
            }
            Err(e) => {
                print!("F({})", &url);
                query_plugins_failures.push((url, e));
            }
        }
        _ = io::stdout().flush();
    }
    println!();

    println!("Fetching and parsing {} plugins...", all_slugs.len());
    let client: Client = reqwest::Client::builder().build().unwrap().into();

    let mut plugin_information_failures = Vec::new();
    for slug in all_slugs {
        let info = client.plugin_information(&slug).await;
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
