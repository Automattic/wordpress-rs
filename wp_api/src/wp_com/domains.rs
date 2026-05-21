use crate::{
    decimal2::Decimal2,
    impl_as_query_value_for_new_type,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    wp_com::{CurrencyCode, WpComSiteId, products::ProductId, segments::SegmentId},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DomainSuggestionsParams {
    /// The search term used to generate domain suggestions.
    pub query: String,
    /// The maximum number of suggestions to return.
    #[uniffi(default = None)]
    pub quantity: Option<u32>,
    /// Restrict suggestions to the given TLDs (e.g. `["com", "net"]`).
    ///
    /// Must be serialized as PHP-style array parameters
    /// (`tlds[]=com&tlds[]=net`). The API's PHP backend silently ignores
    /// CSV (`tlds=com,net`) and honors only the last value when repeated
    /// pairs (`tlds=com&tlds=net`) are used.
    #[uniffi(default = None)]
    pub tlds: Option<Vec<String>>,
    /// Restrict suggestions to a specific vendor (e.g. `"dot"`).
    #[uniffi(default = None)]
    pub vendor: Option<String>,
    /// If `true`, only return WordPress.com subdomain suggestions.
    #[uniffi(default = None)]
    pub only_wordpressdotcom: Option<bool>,
    /// If `true`, include WordPress.com subdomain suggestions alongside
    /// the regular domain suggestions.
    #[uniffi(default = None)]
    pub include_wordpressdotcom: Option<bool>,
    /// If `true`, include `*.home.blog` subdomain suggestions.
    #[uniffi(default = None)]
    pub include_dotblogsubdomain: Option<bool>,
    /// Segment identifier used by the API to tailor suggestions.
    ///
    /// Fetch available segments via the `/wpcom/v2/segments` endpoint.
    /// Note: sending a valid `segment_id` pivots results toward free
    /// `*.wordpress.com` subdomains (and curated themed `.blog`
    /// subdomains for the Blog segment).
    #[uniffi(default = None)]
    pub segment_id: Option<SegmentId>,
}

impl AppendUrlQueryPairs for DomainSuggestionsParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_query_value_pair("query", &self.query)
            .append_option_query_value_pair("quantity", self.quantity.as_ref())
            .append_option_query_value_pair("vendor", self.vendor.as_ref())
            .append_option_query_value_pair(
                "only_wordpressdotcom",
                self.only_wordpressdotcom.as_ref(),
            )
            .append_option_query_value_pair(
                "include_wordpressdotcom",
                self.include_wordpressdotcom.as_ref(),
            )
            .append_option_query_value_pair(
                "include_dotblogsubdomain",
                self.include_dotblogsubdomain.as_ref(),
            )
            .append_option_query_value_pair("segment_id", self.segment_id.as_ref());
        if let Some(tlds) = self.tlds.as_ref() {
            tlds.iter().for_each(|tld| {
                query_pairs_mut.append_pair("tlds[]", tld);
            });
        }
    }
}

/// A domain suggestion returned by the WordPress.com suggestions API.
///
/// The API returns two structurally different shapes in the same array:
/// free WordPress.com subdomains (minimal) and paid domain registrations
/// (full pricing and registration details).
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Enum)]
#[serde(untagged)]
pub enum DomainSuggestion {
    /// A paid domain registration suggestion with full pricing and
    /// registration details.
    Paid(PaidDomainSuggestion),
    /// A free WordPress.com subdomain suggestion.
    Free(FreeDomainSuggestion),
}

/// A free WordPress.com subdomain suggestion (e.g. `"mysite.wordpress.com"`).
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct FreeDomainSuggestion {
    /// The suggested domain name (e.g. `"mysite.wordpress.com"`).
    pub domain_name: String,
    /// Display cost (typically `"Free"`).
    pub cost: String,
    /// Whether this suggestion is free.
    pub is_free: bool,
}

/// A paid domain registration suggestion with full pricing details.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct PaidDomainSuggestion {
    /// The suggested domain name (e.g. `"coolsite.com"`).
    pub domain_name: String,
    /// Relevance score of the suggestion in the range `[0.0, 1.0]`.
    pub relevance: f64,
    /// Whether the domain supports WHOIS privacy protection.
    pub supports_privacy: bool,
    /// The registry vendor (e.g. `"donuts"`, `"dotblogsubdomains"`).
    pub vendor: String,
    /// Reasons why this suggestion was selected (e.g. `"exact-match"`,
    /// `"tld-common"`). Omitted by the API when there are no match reasons.
    pub match_reasons: Option<Vec<String>>,
    /// Maximum number of years the domain can be registered for.
    pub max_reg_years: u32,
    /// Whether the domain supports multi-year registrations.
    pub multi_year_reg_allowed: bool,
    /// WordPress.com product ID used to purchase this domain.
    pub product_id: ProductId,
    /// WordPress.com product slug used to purchase this domain.
    pub product_slug: String,
    /// Formatted registration cost (e.g. `"$18.00"`).
    pub cost: String,
    /// Formatted renewal cost (e.g. `"$18.00"`).
    pub renew_cost: String,
    /// Raw numeric renewal price in `currency_code`.
    pub renew_raw_price: Decimal2,
    /// Raw numeric registration price in `currency_code`.
    pub raw_price: Decimal2,
    /// ISO 4217 currency code for `raw_price`/`renew_raw_price`.
    pub currency_code: CurrencyCode,
    /// Promotional sale price in `currency_code`, if the domain is on sale.
    pub sale_cost: Option<Decimal2>,
    /// `true` if the TLD requires HSTS (e.g. `.dev`).
    pub hsts_required: Option<bool>,
    /// Policy notices attached to the suggestion (e.g. HSTS warnings).
    #[serde(default)]
    #[uniffi(default = [])]
    pub policy_notices: Vec<DomainPolicyNotice>,
}

/// A policy notice attached to a domain suggestion (e.g. an HSTS warning).
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct DomainPolicyNotice {
    /// The notice type identifier (e.g. `"hsts"`).
    #[serde(rename = "type")]
    pub notice_type: String,
    /// Short human-readable label for the notice.
    pub label: String,
    /// Full human-readable message describing the notice.
    pub message: String,
}

/// Optional query parameters for `GET /domains/{name}/is-available/`.
#[derive(Debug, Default, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DomainAvailabilityParams {
    /// Site ID to check domain availability against.
    #[uniffi(default = None)]
    pub blog_id: Option<WpComSiteId>,
    /// Whether this is a pre-check before adding to cart.
    #[uniffi(default = None)]
    pub is_cart_pre_check: Option<bool>,
    /// Vendor for the availability check (e.g. `"100-year-domains"`).
    #[uniffi(default = None)]
    pub vendor: Option<String>,
}

impl AppendUrlQueryPairs for DomainAvailabilityParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("blog_id", self.blog_id.as_ref())
            .append_option_query_value_pair("is_cart_pre_check", self.is_cart_pre_check.as_ref())
            .append_option_query_value_pair("vendor", self.vendor.as_ref());
    }
}

/// Availability status for a domain checked via `GET /domains/{name}/is-available/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum DomainAvailabilityStatus {
    /// Domain is available for registration.
    Available,
    /// Premium domain available at a higher price.
    AvailablePremium,
    /// Domain can be transferred in.
    Transferrable,
    /// Premium domain can be transferred in.
    TransferrablePremium,
    /// Already registered by the same user on a different site.
    RegisteredOnOtherSiteSameUser,
    /// Already mapped to another site by the same user.
    MappedToOtherSiteSameUser,
    /// TLD is not supported for registration.
    TldNotSupported,
    /// TLD is currently in maintenance.
    TldInMaintenance,
    /// Domain is blacklisted.
    BlacklistedDomain,
    /// A status not covered by the known variants.
    #[serde(untagged)]
    Other(String),
}

/// Whether a domain can be mapped to a WordPress.com site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum DomainMappableStatus {
    /// Domain can be mapped.
    Mappable,
    /// Domain is blacklisted and cannot be mapped.
    BlacklistedDomain,
    /// Domain is already mapped to another site.
    MappedDomain,
    /// Domain format is invalid.
    InvalidDomain,
    /// TLD is invalid.
    InvalidTld,
    /// Domain is restricted and cannot be mapped.
    RestrictedDomain,
    /// Domain has a pending transfer.
    TransferPending,
    /// A status not covered by the known variants.
    #[serde(untagged)]
    Other(String),
}

/// Response from `GET /domains/{name}/is-available/` (v1.3).
///
/// Reports whether a domain name is available for registration or
/// mapping, along with pricing and product details when applicable.
///
/// The set of fields present varies by `status`: available domains
/// include full pricing, transferrable domains include partial
/// pricing, and unavailable domains have only the core fields.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct DomainAvailability {
    /// The domain name that was checked.
    pub domain_name: String,
    /// The TLD portion of the domain (e.g. `"com"`, `"io"`, `"dev"`).
    pub tld: String,
    /// Availability status.
    pub status: DomainAvailabilityStatus,
    /// Whether the domain can be mapped to a WordPress.com site.
    pub mappable: DomainMappableStatus,
    /// Whether the domain supports WHOIS privacy protection.
    #[serde(default)]
    #[uniffi(default = false)]
    pub supports_privacy: bool,
    /// Provider of the root domain (`"wpcom"` or `"unknown"`).
    pub root_domain_provider: String,
    /// Product and pricing details. Present for available and
    /// transferrable domains, absent for unavailable domains.
    #[serde(flatten)]
    pub pricing: Option<DomainPricing>,
    /// Match and vendor info. Present for available domains.
    #[serde(flatten)]
    pub match_info: Option<DomainMatchInfo>,
    /// Transfer and mapping details. Present when the domain is
    /// already registered or mapped by the same user on another site.
    #[serde(flatten)]
    pub transfer_info: Option<DomainTransferInfo>,
    /// Type of ownership verification required (e.g.
    /// `"no_verification_required"`).
    pub ownership_verification_type: Option<String>,
    /// `true` if the TLD requires HSTS (e.g. `.dev`).
    pub hsts_required: Option<bool>,
    /// `true` if the `.gay` TLD policy notice is required.
    pub is_dot_gay_notice_required: Option<bool>,
    /// `true` if premium domain transfers are unsupported for this TLD.
    pub cannot_transfer_due_to_unsupported_premium_tld: Option<bool>,
    /// Policy notices attached to the domain (e.g. HSTS warnings).
    #[serde(default)]
    #[uniffi(default = [])]
    pub policy_notices: Vec<DomainPolicyNotice>,
    /// Unix timestamp of when maintenance ends for this domain or TLD.
    pub maintenance_end_time: Option<u64>,
}

/// Product and pricing details for an available or transferrable domain.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct DomainPricing {
    /// WordPress.com product ID for purchasing this domain.
    pub product_id: ProductId,
    /// WordPress.com product slug (e.g. `"domain_reg"`,
    /// `"domain_transfer"`).
    pub product_slug: String,
    /// Formatted registration/transfer cost (e.g. `"$18.00"`).
    pub cost: String,
    /// Raw numeric registration/transfer price in `currency_code`.
    pub raw_price: Decimal2,
    /// ISO 4217 currency code.
    pub currency_code: CurrencyCode,
    /// Formatted renewal cost. Not present for transfer-only domains.
    pub renew_cost: Option<String>,
    /// Raw numeric renewal price in `currency_code`.
    pub renew_raw_price: Option<Decimal2>,
    /// Discounted sale price when a coupon applies.
    pub sale_cost: Option<Decimal2>,
    /// `true` if a premium domain exceeds the price limit.
    pub is_price_limit_exceeded: Option<bool>,
    /// `true` for supported premium domains.
    pub is_supported_premium_domain: Option<bool>,
}

/// Match and vendor information for an available domain.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct DomainMatchInfo {
    /// Reasons the domain matched (e.g. `"exact-match"`,
    /// `"tld-exact"`, `"tld-common"`).
    pub match_reasons: Vec<String>,
    /// The registry vendor (e.g. `"availability"`).
    pub vendor: String,
}

/// Transfer status of a domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum DomainTransferrability {
    /// Domain can be transferred in.
    Transferrable,
    /// Premium domain can be transferred in.
    TransferrablePremium,
    /// A status not covered by the known variants.
    #[serde(untagged)]
    Other(String),
}

/// Transfer and mapping details for a domain registered or mapped
/// by the same user on another site.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct DomainTransferInfo {
    /// Primary domain of the other site where this domain is
    /// registered or mapped.
    pub other_site_domain: String,
    /// Transfer status of this domain.
    pub transferrability: Option<DomainTransferrability>,
}

impl_as_query_value_for_new_type!(DomainName);
uniffi::custom_newtype!(DomainName, String);
/// A domain name (e.g. `"example.com"`, `"myblog.org"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DomainName(pub String);

impl std::fmt::Display for DomainName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl_as_query_value_for_new_type!(CountryCode);
uniffi::custom_newtype!(CountryCode, String);
/// ISO 3166-1 alpha-2 country code (e.g. `"US"`, `"CA"`, `"GB"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CountryCode(pub String);

impl std::fmt::Display for CountryCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for CountryCode {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Structured response from `GET /domains/supported-countries`.
///
/// The raw API response is a flat array where a sentinel entry (empty
/// `code`/`name`, `has_postal_codes: false`) separates "featured" countries
/// from the full alphabetical list. This type deserializes that array and
/// splits it into two vectors, filtering out the sentinel.
///
/// If no sentinel is found the full list is placed in `all` and `featured`
/// is empty.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "Vec<SupportedCountryEntry>")]
pub struct SupportedCountries {
    /// Countries the API surfaces at the top of the picker, in the API's
    /// priority order (not alphabetical).
    pub featured: Vec<SupportedCountry>,
    /// Every supported country, alphabetized by localized name.
    pub all: Vec<SupportedCountry>,
}

impl From<Vec<SupportedCountryEntry>> for SupportedCountries {
    fn from(mut entries: Vec<SupportedCountryEntry>) -> Self {
        let into_countries = |v: Vec<SupportedCountryEntry>| {
            v.into_iter()
                .filter_map(|e| match e {
                    SupportedCountryEntry::Country(c) => Some(c),
                    SupportedCountryEntry::Divider { .. } => None,
                })
                .collect()
        };

        let divider_pos = entries
            .iter()
            .position(|e| matches!(e, SupportedCountryEntry::Divider { .. }));

        match divider_pos {
            Some(pos) => {
                let all_entries = entries.split_off(pos + 1);
                Self {
                    featured: into_countries(entries),
                    all: into_countries(all_entries),
                }
            }
            None => Self {
                featured: Vec::new(),
                all: into_countries(entries),
            },
        }
    }
}

/// Internal type used to deserialize the raw API array which mixes real
/// country entries with sentinel dividers.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum SupportedCountryEntry {
    Country(SupportedCountry),
    Divider {
        #[allow(dead_code)]
        code: String,
        #[allow(dead_code)]
        name: String,
        #[allow(dead_code)]
        has_postal_codes: bool,
    },
}

/// A country supported by the WordPress.com domain registration flow.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct SupportedCountry {
    /// ISO 3166-1 alpha-2 code (e.g. `"US"`).
    pub code: CountryCode,
    /// Localized country name.
    pub name: String,
    /// Whether this country uses postal codes in addresses.
    pub has_postal_codes: bool,
    /// Whether VAT is collected for this country.
    pub vat_supported: bool,
    /// Whether a city is required in the tax address.
    pub tax_needs_city: bool,
    /// Whether a subdivision (state/province) is required in the tax address.
    pub tax_needs_subdivision: bool,
    /// Whether a street address is required for tax purposes.
    #[serde(default)]
    #[uniffi(default = false)]
    pub tax_needs_address: bool,
    /// Whether an organization name is required for tax purposes.
    #[serde(default)]
    #[uniffi(default = false)]
    pub tax_needs_organization: bool,
    /// Additional country codes whose tax rules apply alongside this one.
    #[serde(default)]
    #[uniffi(default = [])]
    pub tax_country_codes: Vec<CountryCode>,
    /// Localized tax name (e.g. `"GST"`, `"VAT"`).
    pub tax_name: Option<String>,
}

/// A state, province, or other subdivision within a supported country.
///
/// Returned from `GET /domains/supported-states/<country_code>`. Countries
/// without subdivision-level address requirements return an empty array.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct SupportedState {
    /// Subdivision code (e.g. `"CA"` for California, `"ON"` for Ontario).
    pub code: String,
    /// Localized subdivision name.
    pub name: String,
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use super::*;
    use rstest::*;

    #[rstest]
    #[case("tests/wpcom/domains/suggestions/basic-query.json", 5)]
    #[case("tests/wpcom/domains/suggestions/larger-quantity.json", 20)]
    #[case("tests/wpcom/domains/suggestions/only-wordpressdotcom.json", 5)]
    #[case("tests/wpcom/domains/suggestions/include-wordpressdotcom.json", 5)]
    #[case("tests/wpcom/domains/suggestions/specific-tlds.json", 5)]
    #[case("tests/wpcom/domains/suggestions/dot-vendor.json", 1)]
    #[case("tests/wpcom/domains/suggestions/photography-niche.json", 5)]
    #[case("tests/wpcom/domains/suggestions/obscure-query.json", 5)]
    fn test_domain_suggestions_deserialization(
        #[case] json_file_path: &str,
        #[case] expected_len: usize,
    ) {
        let file = File::open(json_file_path).expect("Failed to open file");
        let suggestions: Vec<DomainSuggestion> =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(suggestions.len(), expected_len);
    }

    #[test]
    fn test_domain_suggestions_deserialization_basic_query_details() {
        let file = File::open("tests/wpcom/domains/suggestions/basic-query.json")
            .expect("Failed to open file");
        let suggestions: Vec<DomainSuggestion> =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let first = match suggestions
            .first()
            .expect("expected at least one suggestion")
        {
            DomainSuggestion::Paid(paid) => paid,
            DomainSuggestion::Free(_) => panic!("expected Paid variant for first suggestion"),
        };
        assert_eq!(first.domain_name, "brightspace.com");
        assert_eq!(first.relevance, 1.0);
        assert!(first.supports_privacy);
        assert_eq!(first.vendor, "donuts");
        assert_eq!(
            first.match_reasons.as_deref(),
            Some(["tld-common".to_string()].as_slice())
        );
        assert_eq!(first.max_reg_years, 10);
        assert!(first.multi_year_reg_allowed);
        assert_eq!(first.product_id, ProductId(6));
        assert_eq!(first.product_slug, "domain_reg");
        assert_eq!(first.cost, "$18.00");
        assert_eq!(first.renew_cost, "$18.00");
        assert_eq!(first.renew_raw_price.hundredths(), 1800);
        assert_eq!(first.raw_price.hundredths(), 1800);
        assert_eq!(first.currency_code, CurrencyCode("USD".to_string()));
        assert!(first.sale_cost.is_none());
        assert!(first.hsts_required.is_none());
        assert!(first.policy_notices.is_empty());

        // `freshpage.art` has no `match_reasons` field in the JSON.
        let freshpage_art = suggestions
            .iter()
            .find_map(|s| match s {
                DomainSuggestion::Paid(p) if p.domain_name == "freshpage.art" => Some(p),
                _ => None,
            })
            .expect("freshpage.art missing");
        assert!(freshpage_art.match_reasons.is_none());
        assert_eq!(freshpage_art.sale_cost.map(|d| d.hundredths()), Some(164));

        // `testsite.dev` has hsts_required and policy_notices populated.
        let testsite_dev = suggestions
            .iter()
            .find_map(|s| match s {
                DomainSuggestion::Paid(p) if p.domain_name == "testsite.dev" => Some(p),
                _ => None,
            })
            .expect("testsite.dev missing");
        assert_eq!(testsite_dev.hsts_required, Some(true));
        assert_eq!(testsite_dev.policy_notices.len(), 1);
        assert_eq!(testsite_dev.policy_notices[0].notice_type, "hsts");
        assert_eq!(testsite_dev.policy_notices[0].label, "HSTS required");
        assert!(
            testsite_dev.policy_notices[0]
                .message
                .contains("SSL certificate")
        );
    }

    #[test]
    fn test_domain_suggestions_deserialization_only_free() {
        let file = File::open("tests/wpcom/domains/suggestions/only-wordpressdotcom.json")
            .expect("Failed to open file");
        let suggestions: Vec<DomainSuggestion> =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(suggestions.len(), 5);
        for suggestion in &suggestions {
            match suggestion {
                DomainSuggestion::Free(free) => {
                    assert!(free.domain_name.ends_with(".wordpress.com"));
                    assert_eq!(free.cost, "Free");
                    assert!(free.is_free);
                }
                DomainSuggestion::Paid(_) => {
                    panic!("expected only Free variants in only-wordpressdotcom fixture")
                }
            }
        }
    }

    #[test]
    fn test_domain_suggestions_deserialization_mixed() {
        let file = File::open("tests/wpcom/domains/suggestions/include-wordpressdotcom.json")
            .expect("Failed to open file");
        let suggestions: Vec<DomainSuggestion> =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(suggestions.len(), 5);

        let free = match &suggestions[0] {
            DomainSuggestion::Free(f) => f,
            DomainSuggestion::Paid(_) => panic!("expected Free variant for first suggestion"),
        };
        assert!(free.domain_name.ends_with(".wordpress.com"));
        assert!(free.is_free);

        for suggestion in &suggestions[1..] {
            assert!(
                matches!(suggestion, DomainSuggestion::Paid(_)),
                "expected Paid variants after the first suggestion"
            );
        }
    }

    #[test]
    fn test_supported_countries_deserialization() {
        let file = File::open("tests/wpcom/domains/supported_countries/all.json")
            .expect("Failed to open file");
        let response: SupportedCountries =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.featured.len(), 10);
        assert_eq!(response.all.len(), 238);

        // US is in featured and has all optional tax fields populated.
        let us = response
            .featured
            .iter()
            .find(|c| c.code.0 == "US")
            .expect("US missing from featured");
        assert_eq!(us.name, "United States");
        assert!(us.has_postal_codes);
        assert!(!us.vat_supported);
        assert!(!us.tax_needs_city);
        assert!(!us.tax_needs_subdivision);

        // Brazil has no `tax_country_codes` or `tax_name`.
        let br = response
            .all
            .iter()
            .find(|c| c.code.0 == "BR")
            .expect("BR missing from all");
        assert!(br.tax_country_codes.is_empty());
        assert_eq!(br.tax_name, None);

        // Australia has `tax_country_codes` and `tax_name`.
        let au = response
            .all
            .iter()
            .find(|c| c.code.0 == "AU")
            .expect("AU missing from all");
        assert_eq!(au.tax_country_codes, vec![CountryCode::from("AU")]);
        assert_eq!(au.tax_name.as_deref(), Some("GST"));

        // The separator entry should be filtered out.
        let separator = response
            .featured
            .iter()
            .chain(response.all.iter())
            .find(|c| c.code.0.is_empty());
        assert!(separator.is_none(), "separator should be filtered out");
    }

    #[rstest]
    #[case("tests/wpcom/domains/supported_states/us.json", 61)]
    #[case("tests/wpcom/domains/supported_states/ca.json", 13)]
    #[case("tests/wpcom/domains/supported_states/de.json", 0)]
    fn test_supported_states_deserialization(
        #[case] json_file_path: &str,
        #[case] expected_len: usize,
    ) {
        let file = File::open(json_file_path).expect("Failed to open file");
        let states: Vec<SupportedState> =
            serde_json::from_reader(file).expect("Unable to parse JSON");
        assert_eq!(states.len(), expected_len);
        states.iter().for_each(|state| {
            assert!(!state.code.is_empty());
            assert!(!state.name.is_empty());
        });
    }

    #[test]
    fn test_supported_states_deserialization_us_details() {
        let file = File::open("tests/wpcom/domains/supported_states/us.json")
            .expect("Failed to open file");
        let states: Vec<SupportedState> =
            serde_json::from_reader(file).expect("Unable to parse JSON");
        let alabama = states.iter().find(|s| s.code == "AL").expect("AL missing");
        assert_eq!(alabama.name, "Alabama");
    }

    #[test]
    fn test_domain_suggestions_deserialization_dot_vendor() {
        let file = File::open("tests/wpcom/domains/suggestions/dot-vendor.json")
            .expect("Failed to open file");
        let suggestions: Vec<DomainSuggestion> =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(suggestions.len(), 1);
        let only = match &suggestions[0] {
            DomainSuggestion::Paid(p) => p,
            DomainSuggestion::Free(_) => panic!("expected Paid variant for dot-vendor fixture"),
        };
        assert_eq!(only.domain_name, "testsite.home.blog");
        assert_eq!(only.vendor, "dotblogsubdomains");
    }

    #[rstest]
    #[case::available(
        "tests/wpcom/domains/is_available/available.json",
        "freshsite2025.com",
        DomainAvailabilityStatus::Available,
        true
    )]
    #[case::blacklisted(
        "tests/wpcom/domains/is_available/not-available.json",
        "example.com",
        DomainAvailabilityStatus::BlacklistedDomain,
        true
    )]
    #[case::transferrable(
        "tests/wpcom/domains/is_available/transferrable.json",
        "taken-domain.io",
        DomainAvailabilityStatus::Transferrable,
        true
    )]
    #[case::tld_not_supported(
        "tests/wpcom/domains/is_available/tld-not-supported.json",
        "mysite.ai",
        DomainAvailabilityStatus::TldNotSupported,
        false
    )]
    #[case::hsts_required(
        "tests/wpcom/domains/is_available/hsts-required.json",
        "myproject.dev",
        DomainAvailabilityStatus::Other("recent_registration_lock_not_transferrable".to_string()),
        false
    )]
    #[case::available_premium(
        "tests/wpcom/domains/is_available/available-premium.json",
        "luxury.com",
        DomainAvailabilityStatus::AvailablePremium,
        true
    )]
    #[case::mapped_same_user(
        "tests/wpcom/domains/is_available/mapped-same-user.json",
        "myblog.com",
        DomainAvailabilityStatus::MappedToOtherSiteSameUser,
        true
    )]
    #[case::sale_coupon(
        "tests/wpcom/domains/is_available/sale-coupon.json",
        "freshblog2025.online",
        DomainAvailabilityStatus::Available,
        true
    )]
    #[case::maintenance(
        "tests/wpcom/domains/is_available/maintenance.json",
        "mysite.example",
        DomainAvailabilityStatus::TldInMaintenance,
        false
    )]
    #[case::dot_gay_notice(
        "tests/wpcom/domains/is_available/dot-gay-notice.json",
        "testsite2025.gay",
        DomainAvailabilityStatus::Other("mappable".to_string()),
        true
    )]
    fn test_domain_availability_deserialization(
        #[case] json_file_path: &str,
        #[case] expected_domain: &str,
        #[case] expected_status: DomainAvailabilityStatus,
        #[case] expected_privacy: bool,
    ) {
        let file = File::open(json_file_path).expect("Failed to open file");
        let availability: DomainAvailability =
            serde_json::from_reader(file).expect("Unable to parse JSON");
        assert_eq!(availability.domain_name, expected_domain);
        assert_eq!(availability.status, expected_status);
        assert_eq!(availability.supports_privacy, expected_privacy);
    }

    #[test]
    fn test_domain_availability_available_details() {
        let file = File::open("tests/wpcom/domains/is_available/available.json")
            .expect("Failed to open file");
        let availability: DomainAvailability =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let pricing = availability
            .pricing
            .as_ref()
            .expect("available domain should have pricing");
        assert_eq!(pricing.product_id, ProductId(6));
        assert_eq!(pricing.product_slug, "domain_reg");
        assert_eq!(pricing.cost, "$18.00");
        assert_eq!(pricing.raw_price, Decimal2::from_hundredths(1800));
        assert_eq!(pricing.currency_code, CurrencyCode("USD".to_string()));
        assert_eq!(pricing.renew_cost.as_deref(), Some("$18.00"));
        assert_eq!(
            pricing.renew_raw_price,
            Some(Decimal2::from_hundredths(1800))
        );

        let match_info = availability
            .match_info
            .as_ref()
            .expect("available domain should have match info");
        assert_eq!(
            match_info.match_reasons,
            ["exact-match", "tld-exact", "tld-common"].map(String::from)
        );
        assert_eq!(match_info.vendor, "availability");

        assert_eq!(
            availability.ownership_verification_type.as_deref(),
            Some("no_verification_required")
        );
    }

    #[test]
    fn test_domain_availability_blacklisted_has_no_pricing() {
        let file = File::open("tests/wpcom/domains/is_available/not-available.json")
            .expect("Failed to open file");
        let availability: DomainAvailability =
            serde_json::from_reader(file).expect("Unable to parse JSON");
        assert!(availability.pricing.is_none());
        assert!(availability.match_info.is_none());
        assert!(availability.policy_notices.is_empty());
    }

    #[test]
    fn test_domain_availability_transferrable_partial_pricing() {
        let file = File::open("tests/wpcom/domains/is_available/transferrable.json")
            .expect("Failed to open file");
        let availability: DomainAvailability =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let pricing = availability
            .pricing
            .as_ref()
            .expect("transferrable domain should have pricing");
        assert_eq!(pricing.product_id, ProductId(1337));
        assert_eq!(pricing.product_slug, "domain_transfer");
        assert_eq!(pricing.cost, "$48.00");
        assert_eq!(pricing.raw_price, Decimal2::from_hundredths(4800));
        // Transferrable domains don't include renewal pricing.
        assert!(pricing.renew_cost.is_none());
        assert!(pricing.renew_raw_price.is_none());
    }

    #[test]
    fn test_domain_availability_premium_fields() {
        let file = File::open("tests/wpcom/domains/is_available/available-premium.json")
            .expect("Failed to open file");
        let availability: DomainAvailability =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let pricing = availability
            .pricing
            .as_ref()
            .expect("premium domain should have pricing");
        assert_eq!(pricing.is_supported_premium_domain, Some(true));
        assert_eq!(pricing.is_price_limit_exceeded, Some(false));
        assert_eq!(pricing.raw_price, Decimal2::from_hundredths(500000));
    }

    #[test]
    fn test_domain_availability_mapped_same_user() {
        let file = File::open("tests/wpcom/domains/is_available/mapped-same-user.json")
            .expect("Failed to open file");
        let availability: DomainAvailability =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let transfer_info = availability
            .transfer_info
            .as_ref()
            .expect("mapped-same-user should have transfer info");
        assert_eq!(transfer_info.other_site_domain, "myothersite.wordpress.com");
        assert_eq!(
            transfer_info.transferrability,
            Some(DomainTransferrability::Transferrable)
        );
        assert!(availability.pricing.is_none());
    }

    #[test]
    fn test_domain_availability_sale_coupon() {
        let file = File::open("tests/wpcom/domains/is_available/sale-coupon.json")
            .expect("Failed to open file");
        let availability: DomainAvailability =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let pricing = availability
            .pricing
            .as_ref()
            .expect("sale domain should have pricing");
        assert_eq!(pricing.sale_cost, Some(Decimal2::from_hundredths(1000)));
        assert_eq!(pricing.raw_price, Decimal2::from_hundredths(2500));
    }

    #[test]
    fn test_domain_availability_dot_gay_notice() {
        let file = File::open("tests/wpcom/domains/is_available/dot-gay-notice.json")
            .expect("Failed to open file");
        let availability: DomainAvailability =
            serde_json::from_reader(file).expect("Unable to parse JSON");
        assert_eq!(availability.is_dot_gay_notice_required, Some(true));
        assert_eq!(availability.policy_notices.len(), 1);
        assert_eq!(
            availability.policy_notices[0].notice_type,
            "gay_accept_requirements"
        );
    }

    #[test]
    fn test_domain_availability_maintenance() {
        let file = File::open("tests/wpcom/domains/is_available/maintenance.json")
            .expect("Failed to open file");
        let availability: DomainAvailability =
            serde_json::from_reader(file).expect("Unable to parse JSON");
        assert_eq!(availability.maintenance_end_time, Some(1777651200));
        assert!(availability.pricing.is_none());
    }

    #[test]
    fn test_domain_availability_hsts_policy_notices() {
        let file = File::open("tests/wpcom/domains/is_available/hsts-required.json")
            .expect("Failed to open file");
        let availability: DomainAvailability =
            serde_json::from_reader(file).expect("Unable to parse JSON");
        assert_eq!(availability.hsts_required, Some(true));
        assert_eq!(availability.policy_notices.len(), 1);
        assert_eq!(availability.policy_notices[0].notice_type, "hsts");
        assert_eq!(availability.policy_notices[0].label, "HSTS required");
        assert!(
            availability.policy_notices[0]
                .message
                .contains("SSL certificate")
        );
    }
}
