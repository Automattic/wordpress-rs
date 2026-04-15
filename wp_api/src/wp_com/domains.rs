use crate::{
    decimal2::Decimal2,
    impl_as_query_value_for_new_type,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    wp_com::segments::SegmentId,
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
    pub product_id: u64,
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
    /// ISO 4217 currency code for `raw_price`/`renew_raw_price` (e.g. `"USD"`).
    pub currency_code: String,
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

/// A country supported by the WordPress.com domain registration flow.
///
/// Returned from `GET /domains/supported-countries`. Note that the response
/// also contains sentinel entries with empty `code`/`name` used by the web
/// UI as visual separators in the country dropdown.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct SupportedCountry {
    /// ISO 3166-1 alpha-2 code (e.g. `"US"`). Empty for separator entries.
    pub code: String,
    /// Localized country name. Empty for separator entries.
    pub name: String,
    /// Whether this country uses postal codes in addresses.
    pub has_postal_codes: bool,
    /// Whether VAT is collected for this country.
    pub vat_supported: Option<bool>,
    /// Whether a city is required in the tax address.
    pub tax_needs_city: Option<bool>,
    /// Whether a subdivision (state/province) is required in the tax address.
    pub tax_needs_subdivision: Option<bool>,
    /// Whether a street address is required for tax purposes.
    pub tax_needs_address: Option<bool>,
    /// Whether an organization name is required for tax purposes.
    pub tax_needs_organization: Option<bool>,
    /// Additional country codes whose tax rules apply alongside this one.
    pub tax_country_codes: Option<Vec<String>>,
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
        assert_eq!(first.product_id, 6);
        assert_eq!(first.product_slug, "domain_reg");
        assert_eq!(first.cost, "$18.00");
        assert_eq!(first.renew_cost, "$18.00");
        assert_eq!(first.renew_raw_price.hundredths(), 1800);
        assert_eq!(first.raw_price.hundredths(), 1800);
        assert_eq!(first.currency_code, "USD");
        assert_eq!(first.sale_cost, None);
        assert_eq!(first.hsts_required, None);
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
        let countries: Vec<SupportedCountry> =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(countries.len(), 249);

        // US has all optional tax fields populated.
        let us = countries
            .iter()
            .find(|c| c.code == "US")
            .expect("US missing");
        assert_eq!(us.name, "United States");
        assert!(us.has_postal_codes);
        assert_eq!(us.vat_supported, Some(false));
        assert_eq!(us.tax_needs_city, Some(false));
        assert_eq!(us.tax_needs_subdivision, Some(false));

        // Brazil has no `tax_country_codes` or `tax_name`.
        let br = countries
            .iter()
            .find(|c| c.code == "BR")
            .expect("BR missing");
        assert_eq!(br.tax_country_codes, None);
        assert_eq!(br.tax_name, None);

        // Australia has `tax_country_codes` and `tax_name`.
        let au = countries
            .iter()
            .find(|c| c.code == "AU")
            .expect("AU missing");
        assert_eq!(
            au.tax_country_codes.as_deref(),
            Some(["AU".to_string()].as_slice())
        );
        assert_eq!(au.tax_name.as_deref(), Some("GST"));

        // The separator entry has empty code/name and only `has_postal_codes`.
        let separator = countries
            .iter()
            .find(|c| c.code.is_empty())
            .expect("separator entry missing");
        assert!(separator.name.is_empty());
        assert!(!separator.has_postal_codes);
        assert_eq!(separator.vat_supported, None);
        assert_eq!(separator.tax_needs_city, None);
        assert_eq!(separator.tax_needs_subdivision, None);
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
}
