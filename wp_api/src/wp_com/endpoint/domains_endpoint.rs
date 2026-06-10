use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        domains::{
            AllDomainsParams, AllDomainsResponse, CountryCode, DomainAvailability,
            DomainAvailabilityParams, DomainName, DomainSuggestion, DomainSuggestionsParams,
            SetPrimaryDomainParams, SetPrimaryDomainResponse, SiteDomainsResponse,
            SupportedCountries, SupportedState,
        },
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum DomainsRequest {
    #[get(url = "/domains/suggestions", params = &DomainSuggestionsParams, output = Vec<DomainSuggestion>)]
    Suggestions,
    #[get(url = "/domains/supported-countries", output = SupportedCountries)]
    SupportedCountries,
    #[get(url = "/domains/supported-states/<country_code>", output = Vec<SupportedState>)]
    SupportedStates,
    #[get(url = "/domains/<domain_name>/is-available", params = &DomainAvailabilityParams, output = DomainAvailability)]
    IsAvailable,
    #[get(url = "/all-domains", params = &AllDomainsParams, output = AllDomainsResponse)]
    AllDomains,
    #[get(url = "/sites/<wp_com_site_id>/domains", output = SiteDomainsResponse)]
    SiteDomains,
    #[post(url = "/sites/<wp_com_site_id>/domains/primary", params = &SetPrimaryDomainParams, output = SetPrimaryDomainResponse)]
    SetPrimary,
}

impl DerivedRequest for DomainsRequest {
    fn namespace(&self) -> impl AsNamespace {
        match self {
            Self::IsAvailable => WpComNamespace::RestV1_3,
            Self::AllDomains => WpComNamespace::RestV1_2,
            _ => WpComNamespace::RestV1_1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::{
            WpComSiteId,
            domains::{AllDomainsParams, CountryCode, DomainAvailabilityParams, DomainName},
            endpoint::tests::{
                fixture_wp_com_api_url_resolver, validate_wp_com_rest_v1_1_endpoint,
                validate_wp_com_rest_v1_2_endpoint, validate_wp_com_rest_v1_3_endpoint,
            },
            segments::SegmentId,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case::minimal(
        base_domain_suggestions_params(),
        "/domains/suggestions?query=coolsite&quantity=5"
    )]
    #[case::with_tlds(
        DomainSuggestionsParams {
            tlds: Some(vec!["com".to_string(), "net".to_string(), "org".to_string()]),
            ..base_domain_suggestions_params()
        },
        "/domains/suggestions?query=coolsite&quantity=5&tlds%5B%5D=com&tlds%5B%5D=net&tlds%5B%5D=org"
    )]
    #[case::only_wordpressdotcom(
        DomainSuggestionsParams { only_wordpressdotcom: Some(true), ..base_domain_suggestions_params() },
        "/domains/suggestions?query=coolsite&quantity=5&only_wordpressdotcom=true"
    )]
    #[case::include_wordpressdotcom(
        DomainSuggestionsParams { include_wordpressdotcom: Some(true), ..base_domain_suggestions_params() },
        "/domains/suggestions?query=coolsite&quantity=5&include_wordpressdotcom=true"
    )]
    #[case::vendor(
        DomainSuggestionsParams { vendor: Some("dot".to_string()), ..base_domain_suggestions_params() },
        "/domains/suggestions?query=coolsite&quantity=5&vendor=dot"
    )]
    #[case::include_dotblogsubdomain(
        DomainSuggestionsParams { include_dotblogsubdomain: Some(true), ..base_domain_suggestions_params() },
        "/domains/suggestions?query=coolsite&quantity=5&include_dotblogsubdomain=true"
    )]
    #[case::include_dotblogsubdomain_false(
        DomainSuggestionsParams { include_dotblogsubdomain: Some(false), ..base_domain_suggestions_params() },
        "/domains/suggestions?query=coolsite&quantity=5&include_dotblogsubdomain=false"
    )]
    #[case::segment_id(
        DomainSuggestionsParams { segment_id: Some(SegmentId(2)), ..base_domain_suggestions_params() },
        "/domains/suggestions?query=coolsite&quantity=5&segment_id=2"
    )]
    fn suggestions(
        endpoint: DomainsRequestEndpoint,
        #[case] params: DomainSuggestionsParams,
        #[case] expected_path: &str,
    ) {
        validate_wp_com_rest_v1_1_endpoint(endpoint.suggestions(&params), expected_path);
    }

    #[rstest]
    fn supported_countries(endpoint: DomainsRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.supported_countries(),
            "/domains/supported-countries",
        );
    }

    #[rstest]
    #[case::us(CountryCode::from("US"), "/domains/supported-states/US")]
    #[case::ca(CountryCode::from("CA"), "/domains/supported-states/CA")]
    #[case::gb(CountryCode::from("GB"), "/domains/supported-states/GB")]
    fn supported_states(
        endpoint: DomainsRequestEndpoint,
        #[case] country_code: CountryCode,
        #[case] expected_path: &str,
    ) {
        validate_wp_com_rest_v1_1_endpoint(endpoint.supported_states(&country_code), expected_path);
    }

    #[rstest]
    #[case::com(DomainName("example.com".to_string()), "/domains/example.com/is-available?")]
    #[case::org(DomainName("myblog.org".to_string()), "/domains/myblog.org/is-available?")]
    fn is_available(
        endpoint: DomainsRequestEndpoint,
        #[case] domain_name: DomainName,
        #[case] expected_path: &str,
    ) {
        validate_wp_com_rest_v1_3_endpoint(
            endpoint.is_available(&domain_name, &DomainAvailabilityParams::default()),
            expected_path,
        );
    }

    #[rstest]
    fn is_available_with_params(endpoint: DomainsRequestEndpoint) {
        validate_wp_com_rest_v1_3_endpoint(
            endpoint.is_available(
                &DomainName("test.com".to_string()),
                &DomainAvailabilityParams {
                    blog_id: Some(WpComSiteId(12345)),
                    is_cart_pre_check: Some(true),
                    vendor: Some("100-year-domains".to_string()),
                },
            ),
            "/domains/test.com/is-available?blog_id=12345&is_cart_pre_check=true&vendor=100-year-domains",
        );
    }

    #[rstest]
    fn all_domains(endpoint: DomainsRequestEndpoint) {
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.all_domains(&AllDomainsParams::default()),
            "/all-domains?",
        );
    }

    #[rstest]
    fn all_domains_with_garden(endpoint: DomainsRequestEndpoint) {
        validate_wp_com_rest_v1_2_endpoint(
            endpoint.all_domains(&AllDomainsParams {
                garden: Some("starter".to_string()),
            }),
            "/all-domains?garden=starter",
        );
    }

    #[rstest]
    #[case::numeric_id(WpComSiteId(12345), "/sites/12345/domains")]
    #[case::large_id(WpComSiteId(229889220), "/sites/229889220/domains")]
    fn site_domains(
        endpoint: DomainsRequestEndpoint,
        #[case] site_id: WpComSiteId,
        #[case] expected_path: &str,
    ) {
        validate_wp_com_rest_v1_1_endpoint(endpoint.site_domains(&site_id), expected_path);
    }

    #[rstest]
    #[case::numeric_id(WpComSiteId(12345), "/sites/12345/domains/primary")]
    #[case::large_id(WpComSiteId(229889220), "/sites/229889220/domains/primary")]
    fn set_primary(
        endpoint: DomainsRequestEndpoint,
        #[case] site_id: WpComSiteId,
        #[case] expected_path: &str,
    ) {
        validate_wp_com_rest_v1_1_endpoint(endpoint.set_primary(&site_id), expected_path);
    }

    fn base_domain_suggestions_params() -> DomainSuggestionsParams {
        DomainSuggestionsParams {
            query: "coolsite".to_string(),
            quantity: Some(5),
            tlds: None,
            vendor: None,
            only_wordpressdotcom: None,
            include_wordpressdotcom: None,
            include_dotblogsubdomain: None,
            segment_id: None,
        }
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> DomainsRequestEndpoint {
        DomainsRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
