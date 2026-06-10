use crate::{
    date::{WpDateString, deserialize_optional_date_string},
    decimal2::Decimal2,
    impl_as_query_value_for_new_type,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    wp_com::{
        CurrencyCode, WpComSiteId,
        me::WpComUserId,
        products::{ProductId, ProductSlug},
        segments::SegmentId,
        sites::WpComSiteSlug,
        subscribers::SubscriptionId,
    },
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
    pub product_slug: ProductSlug,
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
    pub product_slug: ProductSlug,
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

/// Optional query parameters for `GET /all-domains/` (v1.2).
#[derive(Debug, Default, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AllDomainsParams {
    /// Filter domains by garden name.
    #[uniffi(default = None)]
    pub garden: Option<String>,
}

impl AppendUrlQueryPairs for AllDomainsParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut.append_option_query_value_pair("garden", self.garden.as_ref());
    }
}

/// Response from `GET /all-domains/` (v1.2).
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct AllDomainsResponse {
    /// List of domains across all sites for the authenticated user.
    pub domains: Vec<AllDomainItem>,
}

/// A domain item returned by `GET /all-domains/` (v1.2).
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct AllDomainItem {
    /// The domain name (e.g. `"example.com"`).
    pub domain: DomainName,
    /// Domain subtype indicating how the domain is associated with the site.
    pub subtype: DomainSubtype,
    /// The site ID this domain belongs to.
    pub blog_id: WpComSiteId,
    /// The site name.
    pub blog_name: String,
    /// The site slug used in URLs.
    pub site_slug: WpComSiteSlug,
    /// Whether the domain is configured for automatic renewal.
    pub auto_renewing: bool,
    /// Whether the current authenticated user owns this domain.
    pub current_user_is_owner: bool,
    /// Whether the site only has a domain (no content).
    pub is_domain_only_site: bool,
    /// Expiry date of the domain in `"YYYY-MM-DD"` format, or `None` if it has
    /// no expiry.
    #[serde(default, deserialize_with = "deserialize_optional_date_string")]
    pub expiry: Option<WpDateString>,
    /// Whether the domain has expired.
    pub expired: bool,
    /// Whether this is the primary domain for the site.
    pub primary_domain: bool,
    /// Whether this domain can be set as the site's primary domain.
    pub can_set_as_primary: bool,
    /// Resolved status of the domain.
    pub domain_status: DomainListItemStatus,
    /// Subscription ID for the domain purchase, if any.
    pub subscription_id: Option<SubscriptionId>,
    /// Tags describing domain characteristics
    /// (e.g. `"domain_only"`, `"wpcom_staging"`, `"hundred_year_domain"`).
    #[serde(default)]
    #[uniffi(default = [])]
    pub tags: Vec<String>,
}

/// Domain subtype indicating how the domain is associated with the site.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct DomainSubtype {
    /// Subtype identifier.
    pub id: DomainSubtypeId,
    /// Localized human-readable label.
    pub label: String,
}

/// How a domain is associated with a WordPress.com site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum DomainSubtypeId {
    /// Free WordPress.com address (e.g. `"mysite.wordpress.com"`).
    DefaultAddress,
    /// External domain connected/mapped to the site.
    DomainConnection,
    /// Domain registered through WordPress.com.
    DomainRegistration,
    /// Domain transfer in progress.
    DomainTransfer,
    /// Site redirect to another URL.
    SiteRedirect,
    /// A subtype not covered by the known variants.
    #[serde(untagged)]
    Other(String),
}

/// Resolved status of a domain in `GET /all-domains/` (v1.2).
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct DomainListItemStatus {
    /// Status identifier.
    pub id: DomainListItemStatusId,
    /// Localized human-readable status label.
    pub label: String,
    /// Status severity type.
    #[serde(rename = "type")]
    pub status_type: DomainListItemStatusType,
    /// Call-to-action for the user, if any action is needed.
    pub cta: Option<DomainStatusCta>,
}

/// Call-to-action for a domain status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum DomainStatusCta {
    /// View the domain settings.
    ViewDomain,
    /// View the purchase/subscription.
    ViewPurchase,
    /// View the domain connection setup.
    ViewDomainSetup,
    /// View the domain transfer setup.
    ViewTransferSetup,
    /// A CTA not covered by the known variants.
    #[serde(untagged)]
    Other(String),
}

/// Status identifier for a domain in the all-domains list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum DomainListItemStatusId {
    /// Domain is active and working.
    Active,
    /// Domain setup is in progress.
    InProgress,
    /// Domain is expiring soon.
    ExpiringSoon,
    /// Domain has expired.
    Expired,
    /// Domain is pending renewal.
    PendingRenewal,
    /// Domain registration is pending.
    PendingRegistration,
    /// Domain transfer is in progress.
    PendingTransfer,
    /// Domain transfer has completed.
    TransferCompleted,
    /// Domain transfer encountered an error.
    TransferError,
    /// Domain connection has an error.
    ConnectionError,
    /// A status not covered by the known variants.
    #[serde(untagged)]
    Other(String),
}

/// Severity type for a domain list item status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum DomainListItemStatusType {
    /// Everything is working normally.
    Success,
    /// Attention may be needed soon.
    Warning,
    /// Action is required.
    Error,
    /// Domain mapping not pointing correctly.
    Alert,
    /// Domain is parked (domain-only site with no active status).
    Neutral,
    /// Premium domain.
    Premium,
    /// A status type not covered by the known variants.
    #[serde(untagged)]
    Other(String),
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
///
/// Rejects empty strings during deserialization — an empty code is not
/// a valid country.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CountryCode(pub String);

impl<'de> serde::Deserialize<'de> for CountryCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.is_empty() {
            return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(&s),
                &"a non-empty country code",
            ));
        }
        Ok(Self(s))
    }
}

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

/// Response from `GET /rest/v1.1/sites/{siteId}/domains/`.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct SiteDomainsResponse {
    /// List of domains associated with the site.
    pub domains: Vec<SiteDomain>,
}

/// How a domain is associated with a WordPress.com site.
///
/// Values returned by the `type` field in
/// `GET /rest/v1.1/sites/{siteId}/domains/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum SiteDomainType {
    /// Domain registered through WordPress.com.
    Registered,
    /// Domain mapping to an external domain.
    Mapping,
    /// Domain transfer in progress.
    Transfer,
    /// Site redirect to another URL.
    Redirect,
    /// Free WordPress.com subdomain (e.g. `"mysite.wordpress.com"`).
    Wpcom,
    /// A type not covered by the known variants.
    #[serde(untagged)]
    Other(String),
}

/// SSL certificate provisioning status for a domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum DomainSslStatus {
    /// SSL certificate is provisioned and active.
    Active,
    /// SSL certificate is pending provisioning.
    Pending,
    /// Domain was recently registered; SSL is transitional.
    NewlyRegistered,
    /// SSL is disabled (e.g. domain expired).
    Disabled,
    /// A status not covered by the known variants.
    #[serde(untagged)]
    Other(String),
}

/// Inbound transfer status for a domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum DomainTransferStatus {
    /// Transfer has not yet started.
    PendingStart,
    /// Transfer is awaiting registry approval.
    PendingRegistry,
    /// Transfer is being processed asynchronously.
    PendingAsync,
    /// Transfer has completed.
    Completed,
    /// A status not covered by the known variants.
    #[serde(untagged)]
    Other(String),
}

/// How a mapped domain's DNS is configured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum DomainConnectionMode {
    /// Default DNS configuration recommended by WordPress.com.
    Suggested,
    /// Custom DNS configuration managed by the user.
    Advanced,
    /// A mode not covered by the known variants.
    #[serde(untagged)]
    Other(String),
}

/// Google Workspace email subscription status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum GoogleAppsSubscriptionStatus {
    /// No Google Workspace subscription exists.
    NoSubscription,
    /// Subscription is active.
    Active,
    /// Pending Terms of Service acceptance.
    PendingTosAcceptance,
    /// Subscription is suspended.
    Suspended,
    /// Status could not be determined.
    Unknown,
    /// A status not covered by the known variants.
    #[serde(untagged)]
    Other(String),
}

/// Titan Mail subscription status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum TitanMailSubscriptionStatus {
    /// No Titan Mail subscription exists.
    NoSubscription,
    /// Subscription is active.
    Active,
    /// Subscription is suspended.
    Suspended,
    /// Subscription has been cancelled.
    Cancelled,
    /// A status not covered by the known variants.
    #[serde(untagged)]
    Other(String),
}

/// Per-mailbox cost information for an email subscription.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct EmailCostPerMailbox {
    /// Numeric cost amount.
    pub amount: Decimal2,
    /// ISO 4217 currency code.
    pub currency: CurrencyCode,
    /// Formatted cost string (e.g. `"$6.00"`).
    pub text: String,
}

/// Google Workspace subscription details for a domain.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct GoogleAppsSubscription {
    /// Subscription status.
    pub status: GoogleAppsSubscriptionStatus,
    /// Whether the domain is eligible for an introductory pricing offer.
    pub is_eligible_for_introductory_offer: Option<bool>,
    /// Date the subscription was created, if active.
    #[serde(default, deserialize_with = "deserialize_optional_date_string")]
    pub subscribed_date: Option<WpDateString>,
    /// WordPress.com billing subscription ID, if active.
    pub subscription_id: Option<SubscriptionId>,
    /// WordPress.com user ID of the subscription owner, if active.
    pub owned_by_user_id: Option<WpComUserId>,
    /// Whether Terms of Service acceptance is pending.
    pub pending_tos_acceptance: Option<bool>,
    /// Whether the expected DNS records are present.
    pub has_expected_dns_records: Option<bool>,
    /// Total number of provisioned mailboxes, if active.
    pub total_user_count: Option<u32>,
    /// Product slug for the subscription.
    pub product_slug: Option<ProductSlug>,
    /// Subscription expiry date.
    #[serde(default, deserialize_with = "deserialize_optional_date_string")]
    pub expiry_date: Option<WpDateString>,
    /// Cost per mailbox at initial purchase.
    pub purchase_cost_per_mailbox: Option<EmailCostPerMailbox>,
    /// Cost per mailbox at renewal.
    pub renewal_cost_per_mailbox: Option<EmailCostPerMailbox>,
}

/// Titan Mail subscription details for a domain.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct TitanMailSubscription {
    /// Subscription status.
    pub status: TitanMailSubscriptionStatus,
    /// Whether the domain is eligible for an introductory pricing offer.
    pub is_eligible_for_introductory_offer: Option<bool>,
    /// Order ID for the subscription, if active.
    pub order_id: Option<String>,
    /// Whether the expected DNS records are present.
    pub has_expected_dns_records: Option<bool>,
    /// Maximum number of mailboxes allowed.
    pub maximum_mailbox_count: Option<u32>,
    /// Current number of active mailboxes.
    pub number_of_mailboxes: Option<u32>,
    /// WordPress.com user ID of the subscription owner.
    pub owned_by_user_id: Option<WpComUserId>,
    /// WordPress.com billing subscription ID, if active.
    pub subscription_id: Option<SubscriptionId>,
    /// URL for the Titan webmail interface.
    pub apps_url: Option<String>,
    /// Subscription expiry date.
    #[serde(default, deserialize_with = "deserialize_optional_date_string")]
    pub expiry_date: Option<WpDateString>,
    /// Cost per mailbox at initial purchase.
    pub purchase_cost_per_mailbox: Option<EmailCostPerMailbox>,
    /// Cost per mailbox at renewal.
    pub renewal_cost_per_mailbox: Option<EmailCostPerMailbox>,
    /// Product slug for the subscription.
    pub product_slug: Option<ProductSlug>,
}

/// A domain associated with a specific WordPress.com site, as returned by
/// `GET /rest/v1.1/sites/{siteId}/domains/`.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct SiteDomain {
    /// The domain name (e.g. `"example.com"`).
    pub domain: DomainName,
    /// The site ID this domain belongs to.
    pub blog_id: WpComSiteId,
    /// How the domain is associated with the site.
    #[serde(rename = "type")]
    pub domain_type: SiteDomainType,
    /// Whether this is the primary domain for the site.
    pub primary_domain: Option<bool>,
    /// Whether this is a free WordPress.com subdomain.
    pub wpcom_domain: Option<bool>,
    /// Whether automatic renewal is enabled.
    pub auto_renewing: Option<bool>,
    /// Whether the domain has expired.
    pub expired: Option<bool>,
    /// Expiry date, if applicable.
    #[serde(default, deserialize_with = "deserialize_optional_date_string")]
    pub expiry: Option<WpDateString>,
    /// Whether the domain is expiring soon.
    pub expiry_soon: Option<bool>,
    /// Whether the domain has an active registration.
    pub has_registration: Option<bool>,
    /// Whether WHOIS privacy is enabled on the registration.
    pub has_private_registration: Option<bool>,
    /// Registration date.
    #[serde(default, deserialize_with = "deserialize_optional_date_string")]
    pub registration_date: Option<WpDateString>,
    /// Whether this is a subdomain (e.g. `"blog.example.com"`).
    pub is_subdomain: Option<bool>,
    /// Whether this is a premium domain.
    pub is_premium: Option<bool>,
    /// Whether the domain registrar lock is active.
    pub is_locked: Option<bool>,
    /// Whether the domain can be renewed.
    pub is_renewable: Option<bool>,
    /// Whether the domain is in the redemption grace period.
    pub is_redeemable: Option<bool>,
    /// Whether ICANN verification is pending.
    pub is_pending_icann_verification: Option<bool>,
    /// Whether this is a WordPress.com staging domain.
    pub is_wpcom_staging_domain: Option<bool>,
    /// Whether the domain is eligible for inbound transfer.
    pub is_eligible_for_inbound_transfer: Option<bool>,
    /// Whether the WHOIS contact information can be edited.
    pub is_whois_editable: Option<bool>,
    /// Email address of the domain owner.
    pub owner: Option<String>,
    /// Domain registrar (e.g. `"OPENSRS"`).
    pub registrar: Option<String>,
    /// Product slug for the domain subscription (e.g. `"domain_reg"`).
    pub product_slug: Option<ProductSlug>,
    /// WordPress.com billing subscription ID for the domain purchase.
    pub subscription_id: Option<SubscriptionId>,
    /// Whether domain registration is pending.
    pub pending_registration: Option<bool>,
    /// Whether a domain transfer is pending.
    pub pending_transfer: Option<bool>,
    /// Whether the domain's DNS is pointed to WordPress.com.
    pub points_to_wpcom: Option<bool>,
    /// Whether the domain uses WordPress.com nameservers.
    pub has_wpcom_nameservers: Option<bool>,
    /// Whether a DNS zone exists for this domain.
    pub has_zone: Option<bool>,
    /// Whether this domain can be set as the site's primary domain.
    pub can_set_as_primary: Option<bool>,
    /// Whether the domain supports Domain Connect protocol.
    pub supports_domain_connect: Option<bool>,
    /// Whether GDPR consent management is supported.
    pub supports_gdpr_consent_management: Option<bool>,
    /// Whether transfer approval is supported.
    pub supports_transfer_approval: Option<bool>,
    /// Whether registrar-level domain locking is available.
    pub domain_locking_available: Option<bool>,
    /// Whether the transfer lock on WHOIS update is optional.
    pub transfer_lock_on_whois_update_optional: Option<bool>,
    /// Whether WHOIS privacy protection is available.
    pub privacy_available: Option<bool>,
    /// Whether the domain has been set to private.
    pub private_domain: Option<bool>,
    /// Whether contact information is publicly disclosed.
    pub contact_info_disclosed: Option<bool>,
    /// Whether contact info disclosure settings can be changed.
    pub contact_info_disclosure_available: Option<bool>,
    /// Whether the current user can manage this domain.
    pub current_user_can_manage: Option<bool>,
    /// Whether the current user can add email to this domain.
    pub current_user_can_add_email: Option<bool>,
    /// Whether the current user can create a site from this domain.
    pub current_user_can_create_site_from_domain_only: Option<bool>,
    /// SSL certificate status.
    pub ssl_status: Option<DomainSslStatus>,
    /// Number of email forwards configured.
    pub email_forwards_count: Option<u32>,
    /// Whether this is a newly registered domain.
    pub new_registration: Option<bool>,
    /// Whether a manual transfer is required.
    pub manual_transfer_required: Option<bool>,
    /// Whether this domain was provided by a partner program.
    pub partner_domain: Option<bool>,
    /// A records required for domain mapping, if any.
    pub a_records_required_for_mapping: Option<Vec<String>>,
    /// Auto-renewal date.
    #[serde(default, deserialize_with = "deserialize_optional_date_string")]
    pub auto_renewal_date: Option<WpDateString>,
    /// Subscription ID of the bundled plan, if any.
    pub bundled_plan_subscription_id: Option<String>,
    /// DNS connection mode for mapped domains.
    pub connection_mode: Option<DomainConnectionMode>,
    /// URL to the domain registration agreement.
    pub domain_registration_agreement_url: Option<String>,
    /// Google Workspace email subscription, if configured.
    pub google_apps_subscription: Option<GoogleAppsSubscription>,
    /// Time when domain registration becomes pending, if applicable.
    pub pending_registration_time: Option<String>,
    /// Whether a WHOIS update is pending.
    pub pending_whois_update: Option<bool>,
    /// Date until the domain can be redeemed.
    #[serde(default, deserialize_with = "deserialize_optional_date_string")]
    pub redeemable_until: Option<WpDateString>,
    /// Date until the domain can be renewed.
    #[serde(default, deserialize_with = "deserialize_optional_date_string")]
    pub renewable_until: Option<WpDateString>,
    /// Subdomain portion, if this is a subdomain.
    pub subdomain_part: Option<String>,
    /// Unix timestamp of when TLD maintenance ends.
    pub tld_maintenance_end_time: Option<u64>,
    /// Earliest date the domain can be transferred away.
    #[serde(default, deserialize_with = "deserialize_optional_date_string")]
    pub transfer_away_eligible_at: Option<WpDateString>,
    /// Inbound transfer status.
    pub transfer_status: Option<DomainTransferStatus>,
    /// Titan Mail email subscription, if configured.
    pub titan_mail_subscription: Option<TitanMailSubscription>,
    /// WHOIS fields that cannot be modified, if any.
    pub whois_update_unmodifiable_fields: Option<Vec<String>>,
}

/// Request body for `POST /rest/v1.1/sites/{siteId}/domains/primary/`.
#[derive(Debug, Clone, Serialize, uniffi::Record)]
pub struct SetPrimaryDomainParams {
    /// The domain name to set as the site's primary domain.
    pub domain: DomainName,
}

/// Response from `POST /rest/v1.1/sites/{siteId}/domains/primary/`.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct SetPrimaryDomainResponse {
    /// Whether the primary domain was set successfully.
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use super::*;
    use rstest::*;

    #[rstest]
    #[case("tests/wpcom/domains/all_domains/basic.json", 4)]
    #[case("tests/wpcom/domains/all_domains/mixed-statuses.json", 7)]
    fn test_all_domains_deserialization(#[case] json_file_path: &str, #[case] expected_len: usize) {
        let file = File::open(json_file_path).expect("Failed to open file");
        let response: AllDomainsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");
        assert_eq!(response.domains.len(), expected_len);
    }

    #[test]
    fn test_all_domains_basic_details() {
        let file =
            File::open("tests/wpcom/domains/all_domains/basic.json").expect("Failed to open file");
        let response: AllDomainsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let first = &response.domains[0];
        assert_eq!(first.domain.0, "fake-blog-123.wordpress.com");
        assert_eq!(first.subtype.id, DomainSubtypeId::DefaultAddress);
        assert_eq!(first.blog_id, WpComSiteId(11111));
        assert_eq!(first.blog_name, "Fake Blog 123");
        assert!(!first.auto_renewing);
        assert!(first.expiry.is_none());
        assert!(!first.expired);
        assert!(first.primary_domain);
        assert!(first.subscription_id.is_none());
        assert!(first.tags.is_empty());
        assert_eq!(first.domain_status.id, DomainListItemStatusId::Active);
        assert_eq!(
            first.domain_status.status_type,
            DomainListItemStatusType::Success
        );
        assert!(first.domain_status.cta.is_none());

        // Empty blog_name is a real edge case from the API.
        let empty_name = &response.domains[1];
        assert_eq!(empty_name.blog_name, "");

        let staging = &response.domains[2];
        assert_eq!(staging.tags, vec!["wpcom_staging"]);

        let redirect = &response.domains[3];
        assert_eq!(redirect.subtype.id, DomainSubtypeId::SiteRedirect);
        assert_eq!(redirect.subtype.label, "Site redirect");
        // Date-only `"YYYY-MM-DD"` expiry, the real-world format from the API.
        assert_eq!(
            redirect.expiry,
            Some(WpDateString("2027-01-01".to_string()))
        );
        // The API returns `subscription_id` as a string, not a number.
        assert_eq!(redirect.subscription_id, Some(SubscriptionId(55555)));
    }

    #[test]
    fn test_all_domains_mixed_statuses() {
        let file = File::open("tests/wpcom/domains/all_domains/mixed-statuses.json")
            .expect("Failed to open file");
        let response: AllDomainsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let expiring = &response.domains[0];
        assert_eq!(
            expiring.domain_status.id,
            DomainListItemStatusId::ExpiringSoon
        );
        assert_eq!(
            expiring.domain_status.status_type,
            DomainListItemStatusType::Warning
        );
        assert_eq!(
            expiring.domain_status.cta,
            Some(DomainStatusCta::ViewPurchase)
        );
        assert!(expiring.is_domain_only_site);
        assert_eq!(expiring.tags, vec!["domain_only"]);

        let expired = &response.domains[1];
        assert_eq!(expired.domain_status.id, DomainListItemStatusId::Expired);
        assert_eq!(
            expired.domain_status.status_type,
            DomainListItemStatusType::Error
        );
        assert!(expired.expired);

        let transfer = &response.domains[2];
        assert_eq!(transfer.subtype.id, DomainSubtypeId::DomainTransfer);
        assert_eq!(
            transfer.domain_status.id,
            DomainListItemStatusId::PendingTransfer
        );
        assert_eq!(
            transfer.domain_status.cta,
            Some(DomainStatusCta::ViewTransferSetup)
        );
        assert!(!transfer.can_set_as_primary);

        let century = &response.domains[3];
        assert_eq!(century.tags, vec!["hundred_year_domain"]);
        assert!(century.auto_renewing);

        let mapped = &response.domains[4];
        assert_eq!(
            mapped.domain_status.status_type,
            DomainListItemStatusType::Alert
        );
        assert_eq!(
            mapped.domain_status.cta,
            Some(DomainStatusCta::ViewDomainSetup)
        );

        let parked = &response.domains[5];
        assert_eq!(
            parked.domain_status.status_type,
            DomainListItemStatusType::Neutral
        );
        assert!(parked.is_domain_only_site);

        let premium = &response.domains[6];
        assert_eq!(
            premium.domain_status.status_type,
            DomainListItemStatusType::Premium
        );
    }

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
        assert_eq!(first.product_slug, ProductSlug("domain_reg".to_string()));
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
        assert_eq!(pricing.product_slug, ProductSlug("domain_reg".to_string()));
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
        assert_eq!(
            pricing.product_slug,
            ProductSlug("domain_transfer".to_string())
        );
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

    #[rstest]
    #[case("tests/wpcom/domains/site_domains/basic.json", 2)]
    #[case("tests/wpcom/domains/site_domains/with-email-subscriptions.json", 3)]
    fn test_site_domains_deserialization(
        #[case] json_file_path: &str,
        #[case] expected_len: usize,
    ) {
        let file = File::open(json_file_path).expect("Failed to open file");
        let response: SiteDomainsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");
        assert_eq!(response.domains.len(), expected_len);
    }

    #[test]
    fn test_site_domains_basic_wpcom_subdomain() {
        let file =
            File::open("tests/wpcom/domains/site_domains/basic.json").expect("Failed to open file");
        let response: SiteDomainsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let wpcom = &response.domains[0];
        assert_eq!(wpcom.domain.0, "fake-example-site.wordpress.com");
        assert_eq!(wpcom.blog_id, WpComSiteId(11111));
        assert_eq!(wpcom.domain_type, SiteDomainType::Wpcom);
        assert_eq!(wpcom.primary_domain, Some(false));
        assert_eq!(wpcom.wpcom_domain, Some(true));
        assert_eq!(wpcom.auto_renewing, Some(false));
        assert_eq!(wpcom.expired, Some(false));
        assert!(wpcom.expiry.is_none());
        assert_eq!(wpcom.has_registration, Some(false));
        assert_eq!(wpcom.ssl_status.as_ref(), Some(&DomainSslStatus::Active));
        assert!(wpcom.google_apps_subscription.is_none());
        assert!(wpcom.titan_mail_subscription.is_none());
        assert!(wpcom.subscription_id.is_none());
    }

    #[test]
    fn test_site_domains_basic_registered_domain() {
        let file =
            File::open("tests/wpcom/domains/site_domains/basic.json").expect("Failed to open file");
        let response: SiteDomainsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let registered = &response.domains[1];
        assert_eq!(registered.domain.0, "fake-example.com");
        assert_eq!(registered.domain_type, SiteDomainType::Registered);
        assert_eq!(registered.primary_domain, Some(true));
        assert_eq!(registered.wpcom_domain, Some(false));
        assert_eq!(registered.auto_renewing, Some(true));
        assert_eq!(registered.expired, Some(false));
        assert_eq!(
            registered.expiry,
            Some(WpDateString("2027-03-15".to_string()))
        );
        assert_eq!(registered.expiry_soon, Some(false));
        assert_eq!(registered.has_registration, Some(true));
        assert_eq!(registered.has_private_registration, Some(true));
        assert_eq!(
            registered.registration_date,
            Some(WpDateString("2024-03-15".to_string()))
        );
        assert_eq!(registered.is_renewable, Some(true));
        assert_eq!(registered.is_redeemable, Some(false));
        assert_eq!(registered.is_eligible_for_inbound_transfer, Some(true));
        assert_eq!(registered.is_whois_editable, Some(true));
        assert_eq!(registered.owner.as_deref(), Some("user@fake-example.com"));
        assert_eq!(registered.registrar.as_deref(), Some("OPENSRS"));
        assert_eq!(
            registered.product_slug,
            Some(ProductSlug("domain_reg".to_string()))
        );
        assert_eq!(registered.subscription_id, Some(SubscriptionId(67890)));
        assert_eq!(registered.points_to_wpcom, Some(true));
        assert_eq!(registered.has_wpcom_nameservers, Some(true));
        assert_eq!(registered.has_zone, Some(true));
        assert_eq!(registered.can_set_as_primary, Some(true));
        assert_eq!(registered.domain_locking_available, Some(true));
        assert_eq!(registered.privacy_available, Some(true));
        assert_eq!(registered.contact_info_disclosure_available, Some(true));
        assert_eq!(
            registered.auto_renewal_date,
            Some(WpDateString("2027-03-15".to_string()))
        );
        assert_eq!(
            registered.renewable_until,
            Some(WpDateString("2027-04-15".to_string()))
        );
    }

    #[test]
    fn test_site_domains_google_apps_subscription() {
        let file = File::open("tests/wpcom/domains/site_domains/with-email-subscriptions.json")
            .expect("Failed to open file");
        let response: SiteDomainsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let domain = &response.domains[0];
        assert_eq!(domain.email_forwards_count, Some(2));

        let google = domain
            .google_apps_subscription
            .as_ref()
            .expect("expected google_apps_subscription");
        assert_eq!(google.status, GoogleAppsSubscriptionStatus::Active);
        assert_eq!(google.is_eligible_for_introductory_offer, Some(false));
        assert_eq!(
            google.subscribed_date,
            Some(WpDateString("2024-07-15T10:00:00+00:00".to_string()))
        );
        assert_eq!(google.subscription_id, Some(SubscriptionId(55001)));
        assert_eq!(google.owned_by_user_id, Some(WpComUserId(33001)));
        assert_eq!(google.pending_tos_acceptance, Some(false));
        assert_eq!(google.has_expected_dns_records, Some(true));
        assert_eq!(google.total_user_count, Some(3));
        assert_eq!(
            google.product_slug,
            Some(ProductSlug(
                "wp_google_workspace_business_starter_monthly".to_string()
            ))
        );

        let purchase_cost = google
            .purchase_cost_per_mailbox
            .as_ref()
            .expect("expected purchase_cost_per_mailbox");
        assert_eq!(purchase_cost.amount, Decimal2::from_hundredths(600));
        assert_eq!(purchase_cost.currency, CurrencyCode("USD".to_string()));
        assert_eq!(purchase_cost.text, "$6.00");
    }

    #[test]
    fn test_site_domains_titan_mail_no_subscription() {
        let file = File::open("tests/wpcom/domains/site_domains/with-email-subscriptions.json")
            .expect("Failed to open file");
        let response: SiteDomainsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let domain = &response.domains[0];
        let titan = domain
            .titan_mail_subscription
            .as_ref()
            .expect("expected titan_mail_subscription");
        assert_eq!(titan.status, TitanMailSubscriptionStatus::NoSubscription);
        assert_eq!(titan.is_eligible_for_introductory_offer, Some(true));
        assert_eq!(titan.maximum_mailbox_count, Some(0));
        assert_eq!(titan.owned_by_user_id, Some(WpComUserId(33001)));
    }

    #[test]
    fn test_site_domains_mapped_domain() {
        let file = File::open("tests/wpcom/domains/site_domains/with-email-subscriptions.json")
            .expect("Failed to open file");
        let response: SiteDomainsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let mapped = &response.domains[1];
        assert_eq!(mapped.domain.0, "fake-mapped-site.org");
        assert_eq!(mapped.domain_type, SiteDomainType::Mapping);
        assert_eq!(mapped.primary_domain, Some(false));
        assert_eq!(mapped.ssl_status.as_ref(), Some(&DomainSslStatus::Pending));
        assert_eq!(
            mapped.connection_mode.as_ref(),
            Some(&DomainConnectionMode::Advanced)
        );
        assert_eq!(
            mapped
                .a_records_required_for_mapping
                .as_ref()
                .map(|v| v.iter().map(String::as_str).collect::<Vec<_>>()),
            Some(vec!["192.0.78.24", "192.0.78.25"])
        );
        assert_eq!(mapped.supports_domain_connect, Some(true));
        assert_eq!(mapped.has_wpcom_nameservers, Some(false));
    }

    #[test]
    fn test_site_domains_transfer_domain() {
        let file = File::open("tests/wpcom/domains/site_domains/with-email-subscriptions.json")
            .expect("Failed to open file");
        let response: SiteDomainsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let transfer = &response.domains[2];
        assert_eq!(transfer.domain.0, "fake-transfer-pending.net");
        assert_eq!(transfer.domain_type, SiteDomainType::Transfer);
        assert_eq!(transfer.pending_transfer, Some(true));
        assert_eq!(transfer.can_set_as_primary, Some(false));
        assert_eq!(
            transfer.transfer_status.as_ref(),
            Some(&DomainTransferStatus::PendingRegistry)
        );
        assert!(transfer.ssl_status.is_none());
    }
}
