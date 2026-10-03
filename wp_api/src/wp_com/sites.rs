use crate::{
    JsonValue, impl_as_query_value_from_to_string,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    users::UserCapabilitiesMap,
    wp_com::{
        WpComSiteId,
        me::WpComUserId,
        products::{ProductSlug, ProductType},
    },
    wp_content_string_id,
};

/// Identifies a WordPress.com site by either its numeric ID or its slug.
///
/// Used in API requests that target a specific site, such as fetching site details
/// or requesting OAuth2 authorization scoped to a particular blog.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum WpComSiteIdentifier {
    /// A site identified by its numeric WordPress.com ID.
    Id { value: u64 },
    /// A site identified by its slug (e.g., `"mysite.wordpress.com"`).
    Slug { value: String },
}

impl std::fmt::Display for WpComSiteIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WpComSiteIdentifier::Id { value } => write!(f, "{value}"),
            WpComSiteIdentifier::Slug { value } => write!(f, "{value}"),
        }
    }
}
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wp_derive::WpDeriveParamsField;
use wp_serde_helper::{
    deserialize_false_as_none, deserialize_false_or_string, deserialize_null_as_empty_vec,
    deserialize_u64_or_none_with_zero_as_none, deserialize_u64_or_string,
    deserialize_u64_or_string_or_none_as_t,
};

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)]
pub struct SitesListParams {
    #[uniffi(default = None)]
    pub site_visibility: Option<SiteVisibility>,

    /// Whether to include domain-only sites. Defaults to true if not provided.
    #[uniffi(default = None)]
    pub include_domain_only: Option<bool>,

    /// Whether to include redirect sites. Defaults to true if not provided.
    #[uniffi(default = None)]
    pub include_redirect: Option<bool>,

    /// Whether to include A8C owned sites. Defaults to true if not provided.
    #[uniffi(default = None)]
    pub include_a8c_owned: Option<bool>,

    #[uniffi(default = None)]
    pub site_activity: Option<SiteActivity>,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SiteVisibility {
    /// Return all sites user is a member of, both visible and hidden, along with the sites
    /// [`SiteVisibility::Deleted`] returns.
    All,
    /// Only return sites set to visible for the user.
    Visible,
    /// Only return sites set to hidden for the user.
    Hidden,
    /// Only return deleted sites that are awaiting purge and that the user can manage.
    Deleted,
}

impl_as_query_value_from_to_string!(SiteVisibility);

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SiteActivity {
    All,
    Active,
    Inactive,
}

impl_as_query_value_from_to_string!(SiteActivity);

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPComSiteListResponse {
    pub sites: Vec<WPComSite>,
}

wp_content_string_id!(WpComSiteSlug);

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPComSite {
    /// The site's WordPress.com ID
    #[serde(rename = "ID")]
    pub id: WpComSiteId,

    /// The site's handle – useful in URLs.
    /// Only available from list requests. Member-only.
    pub slug: Option<WpComSiteSlug>,

    /// The site's name as set in Settings > General > Site Title.
    pub name: String,

    /// The site's tagline as set in Settings > General > Tagline.
    pub description: String,

    /// The site's URL as set in https://wordpress.com/domains.
    #[serde(rename = "URL")]
    pub url: String,

    /// The user's capabilities for the site. Member-only.
    pub capabilities: Option<UserCapabilitiesMap>,

    /// Whether the site is a Jetpack site.
    pub jetpack: bool,

    ///  Whether the site is connected to WP.com.
    pub jetpack_connection: bool,

    /// Whether the site is a Multisite site or not. Always true for WP.com sites. Member-only.
    pub is_multisite: Option<bool>,

    /// User ID of the site owner. Member-only. `None` when the site has no administrator.
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none_as_t")]
    pub site_owner: Option<WpComUserId>,

    /// The number of posts the site has.
    pub post_count: u64,

    /// The number of subscribers the site has,
    pub subscribers_count: u64,

    /// Primary language code of the site.
    #[serde(alias = "locale")]
    // It's `lang` in the site list, but `locale` in the site details
    pub lang: String,

    /// The site's icon.
    pub icon: Option<WPComSiteIcon>,

    /// The site logo, set in the Customizer
    pub logo: Option<WPComSiteLogo>,

    /// If this site is visible in the user's site list.
    pub visible: bool,

    /// If the site is a private site or not.
    pub is_private: bool,

    /// If the site is a "coming soon" site or not
    pub is_coming_soon: bool,

    /// Whether the site is single user. Only returned for WP.com sites and for Jetpack sites with version 3.4 or higher.
    pub single_user_site: Option<bool>,

    /// If the site is a VIP site or not. Member-only.
    pub is_vip: Option<bool>,

    /// If the current user is subscribed to this site in the reader.
    pub is_following: bool,

    /// P2 Organization identifier
    #[serde(deserialize_with = "deserialize_u64_or_none_with_zero_as_none")]
    pub organization_id: Option<u64>,

    /// An array of options/settings for the blog. Only viewable by users with post editing rights to the site.
    pub options: Option<HashMap<String, JsonValue>>,

    /// Details of the current plan for this site. Member-only.
    pub plan: Option<WPComPlan>,

    /// Details of the current products for this site. Member-only.
    pub products: Option<Vec<WPComProduct>>,

    /// Site meta data for Zendesk. Member-only.
    pub zendesk_site_meta: Option<WPComZendeskSiteMeta>,

    /// Available updates for the site.
    pub updates: Option<WPComSiteAvailableUpdates>,

    /// A list of active Jetpack modules. Member-only.
    #[serde(default, deserialize_with = "deserialize_null_as_empty_vec")]
    pub jetpack_modules: Vec<String>,

    /// How much space a user has left for uploads
    pub quota: Option<WPComQuota>,

    /// The launch status of the site. `None` when the site has none, as with Jetpack sites that
    /// aren't Atomic.
    #[serde(deserialize_with = "deserialize_false_as_none")]
    pub launch_status: Option<WPComLaunchStatus>,

    /// The migration status of the site.
    pub site_migration: WPComSiteMigrationStatus,

    /// If the site has Full Site Editing active or not.
    pub is_fse_active: bool,

    /// If the site is capable of Full Site Editing or not.
    pub is_fse_eligible: bool,

    /// If the site has the core site editor enabled.
    pub is_core_site_editor_enabled: bool,

    /// If the site is a WP.com Atomic one.
    pub is_wpcom_atomic: bool,

    /// If the site is a WP.com staging site.
    pub is_wpcom_staging_site: bool,

    /// If the site ever used an eCommerce trial. Member-only.
    pub was_ecommerce_trial: Option<bool>,

    /// If the site ever upgraded to a paid plan from a trial. Member-only.
    pub was_upgraded_from_trial: Option<bool>,

    /// If the site ever used a migration trial. Member-only.
    pub was_migration_trial: Option<bool>,

    /// If the site ever used a hosting trial. Member-only.
    pub was_hosting_trial: Option<bool>,

    /// If the site flagged as deleted.
    pub is_deleted: bool,

    /// If the site is an A4A client site.
    pub is_a4a_client: bool,

    /// If the site is an A4A dev site.
    pub is_a4a_dev_site: bool,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPComSiteIcon {
    pub img: Option<String>,
    pub ico: Option<String>,
    pub media_id: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPComSiteLogo {
    #[serde(deserialize_with = "deserialize_u64_or_string")]
    pub id: u64,
    pub sizes: Vec<WPComLogoSize>,
    /// `None` when the logo setting doesn't point at an attachment.
    #[serde(deserialize_with = "deserialize_false_or_string")]
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPComLogoSize {
    pub width: u64,
    pub height: u64,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPComPlan {
    pub product_id: WpComPlanProductId,
    pub product_slug: Option<ProductSlug>,
    pub product_name_short: Option<String>,
    /// Not sent for VIP sites.
    pub expired: Option<bool>,
    /// Not sent for VIP sites.
    pub user_is_owner: Option<bool>,
    /// Not sent for VIP sites.
    pub is_free: Option<bool>,
    pub features: WPComPlanFeatures,
}

/// The product ID of a site's plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, uniffi::Enum)]
pub enum WpComPlanProductId {
    /// VIP sites send `"vip"` in place of a store product ID.
    #[serde(rename = "vip")]
    Vip,
    /// A store product ID.
    #[serde(untagged)]
    Id(u64),
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPComPlanFeatures {
    pub active: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPComProduct {
    #[serde(deserialize_with = "deserialize_u64_or_string")]
    pub product_id: u64,
    pub product_slug: Option<ProductSlug>,
    pub product_name: Option<String>,
    pub product_name_short: Option<String>,
    pub product_type: Option<ProductType>,
    pub expired: bool,
    pub user_is_owner: bool,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPComZendeskSiteMeta {
    /// `None` when WordPress.com has no Zendesk name for the site's plan.
    #[serde(deserialize_with = "deserialize_false_or_string")]
    pub plan: Option<String>,
    pub addon: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPComQuota {
    pub space_allowed: u64,
    pub space_used: u64,
    pub space_available: u64,
    pub percent_used: f64,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum WPComLaunchStatus {
    Unlaunched,
    Launched,
    #[serde(untagged)]
    Unknown(String),
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPComSiteMigrationStatus {
    pub is_complete: bool,
    pub in_progress: bool,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPComSiteAvailableUpdates {
    pub wordpress: u64,
    pub plugins: u64,
    pub themes: u64,
    pub translations: u64,
    pub total: u64,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPComSiteSummary {
    /// The site's WordPress.com ID
    #[serde(rename = "ID")]
    pub id: WpComSiteId,

    /// The site's name as set in Settings > General > Site Title.
    pub name: String,

    /// The site's tagline as set in Settings > General > Tagline.
    pub description: String,

    /// The site's URL as set in https://wordpress.com/domains.
    #[serde(rename = "URL")]
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serde::Deserialize;
    use std::io::Read;

    #[rstest]
    #[case("v1.2-me-sites-01.json", 6)]
    // A deleted site, which is sent without its member-only fields.
    #[case("v1.2-me-sites-02.json", 1)]
    fn test_wpcom_site_list_response_deserialization(
        #[case] json_file_path: &str,
        #[case] expected_count: usize,
    ) {
        let json = test_json(json_file_path).expect("Failed to read JSON file");
        let site_list: WPComSiteListResponse =
            serde_json::from_slice(json.as_slice()).expect("Failed to deserialize user info");
        assert_eq!(site_list.sites.len(), expected_count);
    }

    #[rstest]
    #[case("v1.2-sites-01.json", 100001003)]
    fn test_wpcom_site_single_response_deserialization(
        #[case] json_file_path: &str,
        #[case] expected_id: u64,
    ) {
        let json = test_json(json_file_path).expect("Failed to read JSON file");
        let site: WPComSite =
            serde_json::from_slice(json.as_slice()).expect("Failed to deserialize site data");
        assert_eq!(site.id, crate::wp_com::WpComSiteId(expected_id));
    }

    #[rstest]
    #[case("v1.2-sites-noauth-01.json", 158030780)]
    fn test_wpcom_site_summary_deserialization(
        #[case] json_file_path: &str,
        #[case] expected_id: u64,
    ) {
        let json = test_json(json_file_path).expect("Failed to read JSON file");
        let site: WPComSiteSummary =
            serde_json::from_slice(json.as_slice()).expect("Failed to deserialize site data");
        assert_eq!(site.id, crate::wp_com::WpComSiteId(expected_id));
    }

    fn test_json(input: &str) -> Result<Vec<u8>, std::io::Error> {
        let mut file_path = std::path::PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
        file_path.push("wp_api");
        file_path.push("tests");
        file_path.push("wpcom");
        file_path.push("sites");
        file_path.push(input);

        let mut f = std::fs::File::open(file_path)?;
        let mut buffer = Vec::new();

        // read the whole file
        f.read_to_end(&mut buffer)?;

        Ok(buffer)
    }

    #[derive(Debug, Deserialize)]
    pub struct SiteLaunchStatus {
        #[serde(deserialize_with = "deserialize_false_as_none")]
        pub launch_status: Option<WPComLaunchStatus>,
    }

    #[rstest] // `false` means the site has no launch status, not that it's unlaunched
    #[case(r#"{"launch_status": false}"#, None)]
    #[case(
        r#"{"launch_status": "unlaunched"}"#,
        Some(WPComLaunchStatus::Unlaunched)
    )]
    #[case(r#"{"launch_status": "launched"}"#, Some(WPComLaunchStatus::Launched))]
    fn test_deserialize_launch_status(
        #[case] test_case: &str,
        #[case] expected_result: Option<WPComLaunchStatus>,
    ) {
        let status: SiteLaunchStatus =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, status.launch_status);
    }

    #[rstest] // VIP sites send `"vip"` as the plan's product ID, and omit the subscription fields
    #[case(r#"{"product_id": 1008,"product_slug": "business-bundle","product_name_short": "Business","expired": false,"user_is_owner": true,"is_free": false,"features": {"active": []}}"#, WpComPlanProductId::Id(1008))]
    #[case(r#"{"product_id": "vip","product_slug": "vip","product_name_short": "VIP","product_variation": "vip","supports": [],"features": {"active": []}}"#, WpComPlanProductId::Vip)]
    fn test_deserialize_plan_product_id(
        #[case] test_case: &str,
        #[case] expected_result: WpComPlanProductId,
    ) {
        let plan: WPComPlan =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, plan.product_id);
    }

    #[rstest] // The product ID is often encoded as a string instead of a number
    #[case(r#"{"product_id": "2219","product_slug": "jetpack_stats_yearly","product_name": "Jetpack Stats (Commercial license)","product_name_short": "Stats","product_type": "jetpack","expired": false,"user_is_owner": true}
"#, 2219)]
    #[case(r#"{"product_id": 2219,"product_slug": "jetpack_stats_yearly","product_name": "Jetpack Stats (Commercial license)","product_name_short": "Stats","product_type": "jetpack","expired": false,"user_is_owner": true}
"#, 2219)]
    fn test_deserialize_product(#[case] test_case: &str, #[case] product_id: u64) {
        let product: WPComProduct =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(product_id, product.product_id);
    }

    #[rstest] // The logo ID is often encoded as a string instead of a number
    #[case(r#"{"id": "1449","sizes": [],"url": "example.com/foo.png"}"#, 1449)]
    #[case(r#"{"id": 1449,"sizes": [],"url": "example.com/foo.png"}"#, 1449)]
    fn test_deserialize_logo(#[case] test_case: &str, #[case] media_id: u64) {
        let product: WPComSiteLogo =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(media_id, product.id);
    }

    #[rstest] // The site activity is often encoded as a string instead of a number
    #[case(SiteActivity::Active, "active")]
    #[case(SiteActivity::Inactive, "inactive")]
    #[case(SiteActivity::All, "all")]
    fn test_site_activity_status_to_str(
        #[case] test_case: SiteActivity,
        #[case] expected_result: &str,
    ) {
        assert_eq!(expected_result, test_case.to_string().as_str());
    }

    #[rstest] // The site visibility is often encoded as a string instead of a number
    #[case(SiteVisibility::Visible, "visible")]
    #[case(SiteVisibility::Hidden, "hidden")]
    #[case(SiteVisibility::All, "all")]
    #[case(SiteVisibility::Deleted, "deleted")]
    fn test_site_visibility_status_to_str(
        #[case] test_case: SiteVisibility,
        #[case] expected_result: &str,
    ) {
        assert_eq!(expected_result, test_case.to_string().as_str());
    }
}
