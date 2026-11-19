use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace,
        reader_users::{ReaderUserProfile, WpComUserIdentifier},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum ReaderUsersRequest {
    // A user's public profile, by login or by ID. Needs a signed-in user: signed out it's a 403
    // `authorization_required`. A user who isn't there is a 404 `unknown_user`.
    #[get(url = "/users/<wp_com_user_identifier>", output = ReaderUserProfile)]
    Profile,
}

impl DerivedRequest for ReaderUsersRequest {
    fn namespace(&self) -> impl AsNamespace {
        // The newest version: `rest/v1.2` and `rest/v1.3` have no user profile.
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
            me::WpComUserId,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case::by_login(WpComUserIdentifier::Login { value: "example".to_string() }, "/users/example")]
    #[case::by_id(WpComUserIdentifier::Id { value: WpComUserId(5001) }, "/users/5001")]
    fn profile(
        endpoint: ReaderUsersRequestEndpoint,
        #[case] user: WpComUserIdentifier,
        #[case] expected_path: &str,
    ) {
        validate_wp_com_rest_v1_1_endpoint(endpoint.profile(&user), expected_path);
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ReaderUsersRequestEndpoint {
        ReaderUsersRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
