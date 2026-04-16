use crate::{
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
    pub renew_raw_price: f64,
    /// Raw numeric registration price in `currency_code`.
    pub raw_price: f64,
    /// ISO 4217 currency code for `raw_price`/`renew_raw_price` (e.g. `"USD"`).
    pub currency_code: String,
    /// Promotional sale price in `currency_code`, if the domain is on sale.
    pub sale_cost: Option<f64>,
    /// `true` if the TLD requires HSTS (e.g. `.dev`).
    pub hsts_required: Option<bool>,
    /// Policy notices attached to the suggestion (e.g. HSTS warnings).
    #[serde(default)]
    #[uniffi(default = [])]
    pub policy_notices: Vec<DomainPolicyNotice>,
}

impl PaidDomainSuggestion {
    /// Returns a formatted sale price string matching the server's
    /// `combined_sale_cost_display` format, or `None` if the domain is
    /// not on sale.
    ///
    /// The format uses the currency prefix from [`cost`](Self::cost)
    /// (e.g. `"TL"`, `"$"`), comma thousand-separators, and two decimal
    /// places only when the value has a fractional part.
    ///
    /// Examples: `"TL 47.50"`, `"TL 174"`, `"TL 1,099.35"`, `"$ 18.00"`.
    pub fn sale_cost_display(&self) -> Option<String> {
        let sale = self.sale_cost.filter(|&v| v > 0.0)?;

        let prefix: String = self
            .cost
            .trim_start()
            .chars()
            .take_while(|c| !c.is_ascii_digit())
            .collect::<String>()
            .trim_end()
            .to_string();

        let integer = sale as u64;
        let formatted_int =
            num_format::ToFormattedString::to_formatted_string(&integer, &num_format::Locale::en);

        let formatted = if sale.fract() == 0.0 {
            formatted_int
        } else {
            let fraction = ((sale - integer as f64) * 100.0).round() as u64;
            format!("{formatted_int}.{fraction:02}")
        };

        Some(format!("{prefix} {formatted}"))
    }
}

/// Returns a formatted sale price string for a [PaidDomainSuggestion],
/// matching the server's `combined_sale_cost_display` format.
///
/// Returns `None` if the domain is not on sale. This is a free-standing
/// wrapper around [`PaidDomainSuggestion::sale_cost_display`] for UniFFI
/// export, since UniFFI records do not support methods.
#[uniffi::export]
pub fn paid_domain_suggestion_sale_cost_display(
    suggestion: &PaidDomainSuggestion,
) -> Option<String> {
    suggestion.sale_cost_display()
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
        assert_eq!(first.renew_raw_price, 18.0);
        assert_eq!(first.raw_price, 18.0);
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
        assert_eq!(freshpage_art.sale_cost, Some(1.64));

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

    /// Test cases derived from real WordPress.com `/products/?type=domains`
    /// responses, verifying that [`PaidDomainSuggestion::sale_cost_display`]
    /// matches the server's `combined_sale_cost_display` field exactly.
    #[rstest]
    // Two decimal places
    #[case(Some(25.92), "TL 432", Some("TL 25.92"))]
    // One trailing decimal → padded to two
    #[case(Some(47.5), "TL 475", Some("TL 47.50"))]
    // One trailing decimal (different value)
    #[case(Some(39.3), "TL 786", Some("TL 39.30"))]
    // Two decimals, no padding needed
    #[case(Some(183.28), "TL 2,291", Some("TL 183.28"))]
    // Whole number → no decimals
    #[case(Some(174.0), "TL 580", Some("TL 174"))]
    // Whole number (larger)
    #[case(Some(858.0), "TL 1,144", Some("TL 858"))]
    // Thousands with comma separator and decimals
    #[case(Some(1099.35), "TL 3,141", Some("TL 1,099.35"))]
    // Thousands with comma separator and decimals (larger)
    #[case(Some(4689.65), "TL 13,399", Some("TL 4,689.65"))]
    // Thousands with comma separator, padded decimal
    #[case(Some(1508.4), "TL 2,514", Some("TL 1,508.40"))]
    // No sale cost
    #[case(None, "TL 426", None)]
    // Zero sale cost
    #[case(Some(0.0), "TL 426", None)]
    fn test_sale_cost_display(
        #[case] sale_cost: Option<f64>,
        #[case] cost: &str,
        #[case] expected: Option<&str>,
    ) {
        let suggestion = PaidDomainSuggestion {
            domain_name: "test.com".to_string(),
            relevance: 1.0,
            supports_privacy: true,
            vendor: "donuts".to_string(),
            match_reasons: None,
            max_reg_years: 10,
            multi_year_reg_allowed: true,
            product_id: 6,
            product_slug: "domain_reg".to_string(),
            cost: cost.to_string(),
            renew_cost: cost.to_string(),
            renew_raw_price: 0.0,
            raw_price: 0.0,
            currency_code: "TRY".to_string(),
            sale_cost,
            hsts_required: None,
            policy_notices: vec![],
        };
        assert_eq!(suggestion.sale_cost_display().as_deref(), expected);
    }
}
