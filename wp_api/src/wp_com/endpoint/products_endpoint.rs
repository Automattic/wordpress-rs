use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace,
        products::{ProductMap, ProductsParams},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum ProductsRequest {
    #[get(url = "/products", params = &ProductsParams, output = ProductMap)]
    List,
}

impl DerivedRequest for ProductsRequest {
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
            products::ProductsParams,
        },
    };
    use crate::wp_com::language::WPComLanguage;
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn list_no_filter(endpoint: ProductsRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(endpoint.list(&ProductsParams::default()), "/products?");
    }

    #[rstest]
    fn list_with_type_filter(endpoint: ProductsRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.list(&ProductsParams {
                product_type: Some("domains".to_string()),
                ..Default::default()
            }),
            "/products?type=domains",
        );
    }

    #[rstest]
    fn list_with_locale(endpoint: ProductsRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.list(&ProductsParams {
                locale: Some(WPComLanguage::Spanish),
                ..Default::default()
            }),
            "/products?locale=es",
        );
    }

    #[rstest]
    fn list_with_type_and_locale(endpoint: ProductsRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.list(&ProductsParams {
                product_type: Some("domains".to_string()),
                locale: Some(WPComLanguage::Japanese),
            }),
            "/products?type=domains&locale=ja",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ProductsRequestEndpoint {
        ProductsRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
