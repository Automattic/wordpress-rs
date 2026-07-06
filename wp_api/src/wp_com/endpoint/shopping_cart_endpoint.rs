use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace,
        shopping_cart::{CartKey, CreateShoppingCartParams, ShoppingCart},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum ShoppingCartRequest {
    #[post(url = "/me/shopping-cart/<cart_key>", params = &CreateShoppingCartParams, output = ShoppingCart)]
    Create,
}

impl DerivedRequest for ShoppingCartRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::{
            WpComSiteId,
            endpoint::tests::{
                fixture_wp_com_api_url_resolver, validate_wp_com_rest_v1_1_endpoint,
            },
            shopping_cart::CartKey,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn create_with_site(endpoint: ShoppingCartRequestEndpoint) {
        let cart_key = CartKey::Site {
            id: WpComSiteId(12345),
        };
        validate_wp_com_rest_v1_1_endpoint(endpoint.create(&cart_key), "/me/shopping-cart/12345");
    }

    #[rstest]
    fn create_no_site(endpoint: ShoppingCartRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.create(&CartKey::NoSite),
            "/me/shopping-cart/no-site",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ShoppingCartRequestEndpoint {
        ShoppingCartRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
