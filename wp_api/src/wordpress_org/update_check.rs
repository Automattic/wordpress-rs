use http::header::{CONTENT_TYPE, USER_AGENT};
use http::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use url::form_urlencoded::byte_serialize as url_encode;

use crate::request::{RequestMethod, WpNetworkRequest, WpNetworkRequestBody};
use crate::{ParsedUrl, PluginSlug, PluginStatus, PluginWithViewContext, PluginWpOrgDirectorySlug};

use super::de::deserialize_default_values;
use super::plugin_directory::{Banners, Icons};

#[derive(Debug)]
pub struct UpdateCheckRequest {
    pub wordpress_core_version: String,
    pub site_url: ParsedUrl,
    pub plugins: UpdateCheckRequestPlugins,
}

#[derive(Serialize, Debug)]
pub struct UpdateCheckRequestPlugins {
    pub plugins: HashMap<PluginSlug, InstalledPlugin>,
    pub active: Vec<PluginSlug>,
}

impl UpdateCheckRequest {
    pub fn new(
        wordpress_core_version: String,
        site_url: ParsedUrl,
        plugins: Vec<PluginWithViewContext>,
    ) -> Self {
        let active = plugins
            .iter()
            .filter(|p| p.status == PluginStatus::Active || p.status == PluginStatus::NetworkActive)
            .map(|plugin| plugin.plugin.clone())
            .collect();
        let plugins = plugins
            .into_iter()
            .map(|plugin| (plugin.plugin.clone(), plugin.into()))
            .collect();
        UpdateCheckRequest {
            wordpress_core_version,
            site_url,
            plugins: UpdateCheckRequestPlugins { plugins, active },
        }
    }

    fn url_encoded_body(&self) -> Result<Vec<u8>, serde_json::Error> {
        // https://github.com/WordPress/wordpress-develop/blob/6.7.1/src/wp-includes/update.php#L417-L426

        let key_value_pairs = [
            ("plugins", serde_json::to_vec(&self.plugins)?),
            ("all", "true".to_string().into()),
        ];

        let body = key_value_pairs
            .iter()
            .map(|(key, value)| {
                url_encode(key.as_bytes())
                    .chain(["="])
                    .chain(url_encode(value))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("&");
        Ok(body.into())
    }
}

impl TryFrom<UpdateCheckRequest> for WpNetworkRequest {
    type Error = serde_json::Error;

    fn try_from(request: UpdateCheckRequest) -> Result<Self, Self::Error> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            format!(
                "WordPress/{}; {}",
                request.wordpress_core_version,
                request.site_url.url()
            )
            .parse()
            .expect("The user agent value is always valid"),
        );
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );

        let body = request.url_encoded_body()?;

        Ok(WpNetworkRequest {
            method: RequestMethod::POST,
            url: crate::request::endpoint::WpEndpointUrl(
                "https://api.wordpress.org/plugins/update-check/1.1/".to_string(),
            ),
            header_map: Arc::new(headers.into()),
            body: Some(WpNetworkRequestBody::new(body).into()),
        })
    }
}

#[derive(Serialize, Debug)]
pub struct InstalledPlugin {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "PluginURI")]
    pub plugin_uri: String,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Author")]
    pub author: String,
    #[serde(rename = "AuthorURI")]
    pub author_uri: String,
    #[serde(rename = "TextDomain")]
    pub text_domain: String,
    #[serde(rename = "Network")]
    pub network: bool,
    #[serde(rename = "RequiresWP")]
    pub requires_wp: String,
    #[serde(rename = "RequiresPHP")]
    pub requires_php: String,
}

impl From<PluginWithViewContext> for InstalledPlugin {
    fn from(plugin: PluginWithViewContext) -> Self {
        InstalledPlugin {
            name: plugin.name,
            plugin_uri: plugin.plugin_uri,
            version: plugin.version,
            author: plugin.author,
            author_uri: plugin.author_uri,
            text_domain: plugin.textdomain,
            network: plugin.network_only,
            requires_wp: plugin.requires_wp,
            requires_php: plugin.requires_php,
        }
    }
}

#[derive(Deserialize, Debug, uniffi::Record)]
pub struct UpdateCheckResponse {
    #[serde(deserialize_with = "deserialize_default_values")]
    plugins: HashMap<String, UpdateCheckPluginInfo>,
}

#[derive(Deserialize, Debug, uniffi::Record)]
pub struct UpdateCheckPluginInfo {
    pub id: String,
    pub slug: PluginWpOrgDirectorySlug,
    pub plugin: PluginSlug,
    pub new_version: String,
    pub url: String,
    pub package: String,
    pub icons: Option<Icons>,
    #[serde(deserialize_with = "deserialize_default_values")]
    #[serde(default)]
    pub banners: Banners,
    #[serde(deserialize_with = "deserialize_default_values")]
    #[serde(default)]
    pub banners_rtl: Banners,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub requires: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub tested: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub requires_php: String,
    // The `compatibility` field does not seem to be used by WordPress. The only
    // usage I can find is removing it from the .org HTTP API response.
    // https://github.com/WordPress/wordpress-develop/blob/6.7/src/wp-includes/update.php#L560
    // compatibility: Vec<...>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_method() {
        let request = UpdateCheckRequest::new(
            "5.8".to_string(),
            ParsedUrl::parse("https://example.com").unwrap(),
            vec![],
        );
        let request: Result<WpNetworkRequest, _> = request.try_into();
        let request = request.unwrap();
        assert_eq!(request.method, RequestMethod::POST);
    }

    #[test]
    fn test_request_wordpress_version() {
        let request = UpdateCheckRequest::new(
            "5.8".to_string(),
            ParsedUrl::parse("https://example.com").unwrap(),
            vec![],
        );
        let request: Result<WpNetworkRequest, _> = request.try_into();
        let request = request.unwrap();
        let headers = request.header_map.as_header_map();
        assert!(headers.contains_key("User-Agent"));

        let user_agent = headers.get("User-Agent").unwrap().to_str().unwrap();
        assert!(user_agent.contains("WordPress/5.8"));
        assert!(user_agent.contains("https://example.com"));
    }

    #[test]
    fn test_request_url() {
        let request = UpdateCheckRequest::new(
            "5.8".to_string(),
            ParsedUrl::parse("https://example.com").unwrap(),
            vec![],
        );
        let request: Result<WpNetworkRequest, _> = request.try_into();
        let request = request.unwrap();
        assert_eq!(
            request.url.0,
            "https://api.wordpress.org/plugins/update-check/1.1/".to_string()
        );
    }

    #[test]
    fn test_request_body() {
        let plugins_json = r#"
            [
                {
                    "plugin": "akismet/akismet",
                    "status": "inactive",
                    "name": "Akismet Anti-spam: Spam Protection",
                    "plugin_uri": "https://akismet.com/",
                    "author": "Automattic - Anti-spam Team",
                    "author_uri": "https://automattic.com/wordpress-plugins/",
                    "description": {
                    "raw": "raw",
                    "rendered": "rendered"
                    },
                    "version": "5.3.5",
                    "network_only": false,
                    "requires_wp": "5.8",
                    "requires_php": "5.6.20",
                    "textdomain": "akismet"
                },
                {
                    "plugin": "hello-dolly/hello",
                    "status": "active",
                    "name": "Hello Dolly",
                    "plugin_uri": "http://wordpress.org/plugins/hello-dolly/",
                    "author": "Matt Mullenweg",
                    "author_uri": "http://ma.tt/",
                    "description": {
                    "raw": "raw",
                    "rendered": "rendered"
                    },
                    "version": "1.7.2",
                    "network_only": false,
                    "requires_wp": "",
                    "requires_php": "",
                    "textdomain": "hello-dolly"
                }
            ]
        "#;
        let plugins = serde_json::from_str::<Vec<PluginWithViewContext>>(plugins_json).unwrap();
        let request = UpdateCheckRequest::new(
            "5.8".to_string(),
            ParsedUrl::parse("https://example.com").unwrap(),
            plugins,
        );
        let request: Result<WpNetworkRequest, _> = request.try_into();
        let body = request.unwrap().body.unwrap().contents();
        let body_string = String::from_utf8(body).unwrap();
        let body_map = body_string
            .split("&")
            .flat_map(|x| url::form_urlencoded::parse(x.as_bytes()))
            .collect::<HashMap<_, _>>();
        assert!(body_map.contains_key("plugins"));
        assert!(body_map.contains_key("all"));

        let body_plugins_json = body_map
            .get("plugins")
            .and_then(|x| serde_json::from_str::<serde_json::Value>(x).ok())
            .unwrap();

        let plugins_json = body_plugins_json
            .as_object()
            .unwrap()
            .get("plugins")
            .unwrap()
            .as_object()
            .unwrap();
        assert_eq!(plugins_json.len(), 2);
        assert!(plugins_json.contains_key("akismet/akismet"));
        assert!(plugins_json.contains_key("hello-dolly/hello"));

        let active_json = body_plugins_json
            .as_object()
            .unwrap()
            .get("active")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(active_json.len(), 1);
        assert_eq!(active_json[0].as_str().unwrap(), "hello-dolly/hello");

        assert_eq!(body_map.get("all").unwrap(), "true");
    }
}
