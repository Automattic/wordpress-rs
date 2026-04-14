use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace,
        domains::{DomainSuggestion, DomainSuggestionsParams},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum DomainsRequest {
    #[get(url = "/domains/suggestions", params = &DomainSuggestionsParams, output = Vec<DomainSuggestion>)]
    Suggestions,
}

impl DerivedRequest for DomainsRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::{
            endpoint::tests::{
                fixture_wp_com_api_url_resolver, validate_wp_com_rest_v1_1_endpoint,
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
