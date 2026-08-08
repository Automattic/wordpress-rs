use crate::{
    date::WpGmtDateTime,
    decimal2::Decimal2,
    url_query::{AppendUrlQueryPairs, AsQueryValue, QueryPairs, QueryPairsExtension},
    wp_com::{CurrencyCode, TimeSpanUnit, language::WPComLanguage},
    wp_content_string_id,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

uniffi::custom_newtype!(ProductId, u64);
/// WordPress.com product identifier.
///
/// Deserializes from both numeric (`6`) and string (`"6"`) representations,
/// since the API is inconsistent about the encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct ProductId(pub u64);

impl<'de> Deserialize<'de> for ProductId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        wp_serde_helper::deserialize_u64_or_string(deserializer).map(Self)
    }
}

impl std::fmt::Display for ProductId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Product category as classified by the billing system (e.g. `"domain_reg"`,
// `"bundle"`, `"jetpack"`, `"theme"`). The set is open-ended — products define
// their own type — so this is a newtype rather than an enum.
wp_content_string_id!(ProductType);

uniffi::custom_newtype!(ProductSlug, String);
/// WordPress.com product slug (e.g. `"domain_reg"`, `"personal-bundle"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProductSlug(pub String);

impl std::fmt::Display for ProductSlug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

uniffi::custom_newtype!(ProductTierId, u64);
/// Identifies a tier in the product catalog — the group that ties a plan's
/// monthly, yearly and multi-year variants together.
///
/// Deserializes from both numeric (`625`) and string (`"625"`) representations,
/// because the API also uses it as a JSON object key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct ProductTierId(pub u64);

impl<'de> Deserialize<'de> for ProductTierId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        wp_serde_helper::deserialize_u64_or_string(deserializer).map(Self)
    }
}

impl std::fmt::Display for ProductTierId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

uniffi::custom_newtype!(BillPeriodDays, i32);
/// Length of a billing period, in days.
///
/// `-1` marks a product that is never billed, such as the free plan. Monthly
/// products use `31`, and multi-year products use `730`, `1095` or `36500`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BillPeriodDays(pub i32);

impl std::fmt::Display for BillPeriodDays {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Filter for the `type` query parameter on `GET /products`.
///
/// The API supports `"domains"` and `"jetpack"` as built-in filters.
/// Use `Other` for any value not covered by these variants.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ProductTypeFilter {
    /// Return only domain-related products (registrations, transfers, mapping, etc.).
    Domains,
    /// Return only Jetpack plans and products.
    Jetpack,
    /// A product type filter not covered by the known variants.
    Other { value: String },
}

impl AsQueryValue for ProductTypeFilter {
    fn as_query_value(&self) -> impl AsRef<str> {
        match self {
            Self::Domains => "domains".to_string(),
            Self::Jetpack => "jetpack".to_string(),
            Self::Other { value } => value.clone(),
        }
    }
}

/// Parameters for `GET /products`.
#[derive(Debug, Default, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProductsParams {
    /// Filter by product type.
    #[uniffi(default = None)]
    pub product_type: Option<ProductTypeFilter>,
    /// Locale for localized product names and descriptions.
    #[uniffi(default = None)]
    pub locale: Option<WPComLanguage>,
}

impl AppendUrlQueryPairs for ProductsParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("type", self.product_type.as_ref())
            .append_option_query_value_pair("locale", self.locale.as_ref());
    }
}

/// Billing interval for a product.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
pub enum ProductTerm {
    #[serde(rename = "month")]
    Month,
    #[serde(rename = "year")]
    Year,
    #[serde(rename = "two years")]
    TwoYears,
    #[serde(rename = "three years")]
    ThreeYears,
    #[serde(rename = "hundred years")]
    HundredYears,
    #[serde(rename = "one time")]
    OneTime,
    /// A billing term not covered by the known variants.
    #[serde(untagged)]
    Other(String),
}

/// Map of product slug to product, as returned by `GET /products`.
pub type ProductMap = HashMap<String, Product>;

/// A WordPress.com product.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct Product {
    pub product_id: ProductId,
    pub product_name: String,
    pub product_slug: ProductSlug,
    pub description: String,
    pub product_type: ProductType,
    pub available: bool,
    pub billing_product_slug: ProductSlug,
    pub is_domain_registration: bool,
    /// Formatted display cost (e.g. `"$18.00"`), localized to the account's
    /// currency.
    pub cost_display: String,
    /// Formatted combined cost without decimal places (e.g. `"$18"`).
    pub combined_cost_display: String,
    /// Numeric cost in the account's currency.
    pub cost: Decimal2,
    /// Cost in the smallest currency unit (e.g. cents).
    pub cost_smallest_unit: u64,
    pub currency_code: CurrencyCode,
    /// Billing period.
    pub product_term: ProductTerm,
    /// Localized billing period label.
    pub product_term_localized: String,
    pub price_tier_slug: String,
    #[serde(default)]
    #[uniffi(default = [])]
    pub price_tier_list: Vec<PriceTier>,
    /// Domain-specific fields, present only for domain registration products.
    #[serde(flatten)]
    pub domain_info: Option<DomainProductInfo>,
    /// Formatted monthly cost (e.g. `"$1.50"`).
    #[serde(default)]
    #[uniffi(default = None)]
    pub cost_per_month_display: Option<String>,
    /// Numeric sale price when a coupon applies.
    #[serde(default)]
    #[uniffi(default = None)]
    pub sale_cost: Option<Decimal2>,
    /// Formatted combined sale cost (e.g. `"$6.00"`).
    #[serde(default)]
    #[uniffi(default = None)]
    pub combined_sale_cost_display: Option<String>,
    /// Active sale coupon details, if any.
    #[serde(default)]
    #[uniffi(default = None)]
    pub sale_coupon: Option<SaleCoupon>,
    /// Introductory offer details, if any.
    #[serde(default)]
    #[uniffi(default = None)]
    pub introductory_offer: Option<IntroductoryOffer>,
}

/// Domain-specific product metadata, only populated for domain registration
/// products.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct DomainProductInfo {
    /// Top-level domain (e.g. `"com"`, `"net"`).
    pub tld: String,
    /// Whether WHOIS privacy can be purchased with this domain.
    pub is_privacy_protection_product_purchase_allowed: bool,
    /// Whether HSTS is required for this TLD (e.g. `.dev`, `.app`).
    #[serde(default)]
    #[uniffi(default = false)]
    pub is_hsts_required: bool,
    /// Whether the `.gay` TLD policy notice is required.
    #[serde(default)]
    #[uniffi(default = false)]
    pub is_dot_gay_notice_required: bool,
}

/// A pricing tier for usage-based products.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct PriceTier {
    pub minimum_units: u64,
    /// `None` for the highest (unbounded) tier.
    pub maximum_units: Option<u64>,
    pub minimum_price: u64,
    pub maximum_price: u64,
    pub minimum_price_display: String,
    pub minimum_price_monthly_display: String,
    /// `None` for the highest (unbounded) tier.
    pub maximum_price_display: Option<String>,
    /// `None` for the highest (unbounded) tier.
    pub maximum_price_monthly_display: Option<String>,
    #[serde(default)]
    #[uniffi(default = None)]
    pub flat_fee: Option<u64>,
    #[serde(default)]
    #[uniffi(default = None)]
    pub per_unit_fee: Option<u64>,
    #[serde(default)]
    #[uniffi(default = None)]
    pub transform_quantity_divide_by: Option<u64>,
    #[serde(default)]
    #[uniffi(default = None)]
    pub transform_quantity_round: Option<String>,
}

/// Details of an active sale coupon applied to a product.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct SaleCoupon {
    pub start_date: WpGmtDateTime,
    pub expires: WpGmtDateTime,
    /// Discount percentage (e.g. `65` means 65% off).
    pub discount: u32,
    pub product_ids: Vec<ProductId>,
    #[serde(default)]
    #[uniffi(default = [])]
    pub purchase_types: Vec<u32>,
    pub allowed_for_domain_transfers: bool,
    pub allowed_for_renewals: bool,
    pub allowed_for_new_purchases: bool,
    pub code: String,
    #[serde(default)]
    #[uniffi(default = None)]
    pub tld_rank: Option<f64>,
}

/// Introductory pricing offer for a product.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct IntroductoryOffer {
    /// Unit of the offer interval.
    pub interval_unit: TimeSpanUnit,
    pub interval_count: u32,
    #[serde(default)]
    #[uniffi(default = None)]
    pub usage_limit: Option<u32>,
    /// Cost per interval during the offer period.
    pub cost_per_interval: Decimal2,
    pub transition_after_renewal_count: u32,
    pub should_prorate_when_offer_ends: bool,
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use super::*;

    #[test]
    fn test_products_domains_deserialization() {
        let file = File::open("tests/wpcom/products/domains.json").expect("Failed to open file");
        let products: ProductMap = serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(products.len(), 5);

        // domain_map is a non-registration product without domain_info.
        let domain_map = products.get("domain_map").expect("domain_map missing");
        assert_eq!(domain_map.product_id, ProductId(1001));
        assert!(!domain_map.is_domain_registration);
        assert!(domain_map.domain_info.is_none());

        // domain_reg is a registration product with domain_info.
        let domain_reg = products.get("domain_reg").expect("domain_reg missing");
        assert!(domain_reg.is_domain_registration);
        let domain_reg_info = domain_reg
            .domain_info
            .as_ref()
            .expect("domain_reg should have domain_info");
        assert_eq!(domain_reg_info.tld, "com");
        assert!(domain_reg_info.is_privacy_protection_product_purchase_allowed);

        // dotdev_domain requires HSTS.
        let dotdev = products
            .get("dotdev_domain")
            .expect("dotdev_domain missing");
        let dotdev_info = dotdev
            .domain_info
            .as_ref()
            .expect("dotdev_domain should have domain_info");
        assert!(dotdev_info.is_hsts_required);

        // dotgay_domain requires the .gay policy notice.
        let dotgay = products
            .get("dotgay_domain")
            .expect("dotgay_domain missing");
        let dotgay_info = dotgay
            .domain_info
            .as_ref()
            .expect("dotgay_domain should have domain_info");
        assert!(dotgay_info.is_dot_gay_notice_required);
    }

    #[test]
    fn test_products_with_sale_coupon() {
        let file = File::open("tests/wpcom/products/domains.json").expect("Failed to open file");
        let products: ProductMap = serde_json::from_reader(file).expect("Unable to parse JSON");

        let dotinfo = products
            .get("dotinfo_domain")
            .expect("dotinfo_domain missing");
        let coupon = dotinfo
            .sale_coupon
            .as_ref()
            .expect("dotinfo_domain should have sale_coupon");
        assert_eq!(coupon.discount, 65);
        assert_eq!(coupon.code, "fakecoupon123");
        assert_eq!(dotinfo.sale_cost, Some(Decimal2::from_hundredths(700)));
    }

    #[test]
    fn test_products_all_deserialization() {
        let file = File::open("tests/wpcom/products/all.json").expect("Failed to open file");
        let products: ProductMap = serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(products.len(), 7);

        // Verify a product with price tiers (including an unbounded top tier).
        let storage = products
            .get("fake_storage_addon_yearly")
            .expect("fake_storage_addon_yearly missing");
        assert_eq!(storage.price_tier_list.len(), 2);
        assert_eq!(storage.price_tier_list[0].minimum_units, 0);
        assert!(
            storage.price_tier_list[1].maximum_units.is_none(),
            "top tier should be unbounded"
        );

        // Verify a product with an introductory offer.
        let mail = products
            .get("fake_mail_monthly")
            .expect("fake_mail_monthly missing");
        let offer = mail
            .introductory_offer
            .as_ref()
            .expect("fake_mail_monthly should have introductory_offer");
        assert_eq!(offer.interval_unit, TimeSpanUnit::Month);
        assert_eq!(offer.interval_count, 3);
    }

    #[test]
    fn test_products_locale_es_deserialization() {
        let file =
            File::open("tests/wpcom/products/all-locale-es.json").expect("Failed to open file");
        let products: ProductMap = serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(products.len(), 3);

        // Localized product name.
        let domain_map = products.get("domain_map").expect("domain_map missing");
        assert_eq!(domain_map.product_name, "Conexión de dominio falso");
        assert_eq!(domain_map.product_term_localized, "año");
    }

    #[test]
    fn test_products_locale_ja_domains_deserialization() {
        let file =
            File::open("tests/wpcom/products/domains-locale-ja.json").expect("Failed to open file");
        let products: ProductMap = serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(products.len(), 2);

        let domain_map = products.get("domain_map").expect("domain_map missing");
        assert_eq!(domain_map.product_name, "偽ドメイン連携");
        assert_eq!(domain_map.product_term_localized, "年");

        // Domain registration fields still present with locale.
        let domain_reg = products.get("domain_reg").expect("domain_reg missing");
        assert!(domain_reg.is_domain_registration);
        assert_eq!(
            domain_reg
                .domain_info
                .as_ref()
                .expect("domain_reg should have domain_info")
                .tld,
            "com"
        );
    }

    #[test]
    fn test_products_jetpack_deserialization() {
        let file = File::open("tests/wpcom/products/jetpack.json").expect("Failed to open file");
        let products: ProductMap = serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(products.len(), 3);

        // Basic jetpack product without introductory offer.
        let backup = products
            .get("fake_jetpack_backup_daily")
            .expect("fake_jetpack_backup_daily missing");
        assert_eq!(backup.product_type, ProductType("jetpack".to_string()));
        assert!(!backup.is_domain_registration);
        assert!(backup.introductory_offer.is_none());

        // Jetpack product with introductory offer.
        let security = products
            .get("fake_jetpack_security_yearly")
            .expect("fake_jetpack_security_yearly missing");
        assert_eq!(security.product_type, ProductType("jetpack".to_string()));
        let offer = security
            .introductory_offer
            .as_ref()
            .expect("should have introductory_offer");
        assert_eq!(offer.interval_unit, TimeSpanUnit::Year);
        assert_eq!(offer.cost_per_interval, Decimal2::from_hundredths(12000));

        // Bundle product also returned by jetpack filter.
        let complete = products
            .get("fake_jetpack_complete")
            .expect("fake_jetpack_complete missing");
        assert_eq!(complete.product_type, ProductType("bundle".to_string()));
    }
}
