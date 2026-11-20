use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace,
        reader_users::{
            ReaderUserProfile, ReaderUserSuggestionsParams, ReaderUserSuggestionsResponse,
            WpComUserIdentifier,
        },
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum ReaderUsersRequest {
    // A user's public profile, by login or by ID. Needs a signed-in user: signed out it's a 403
    // `authorization_required`. A user who isn't there is a 404 `unknown_user`.
    #[get(url = "/users/<wp_com_user_identifier>", output = ReaderUserProfile)]
    Profile,
    // The people of a site to suggest when a reader types `@` in a comment, as both WordPress apps
    // and the web's notifications panel ask for them. Signed in, it answers for any site: its
    // users, such as only the owner on a site of one, and 175 on the WordPress.com Blog, which the
    // account isn't a member of. Signed out it answers 200 with no suggestions, for any site.
    // Measured on 2026-11-21.
    #[get(url = "/users/suggest", params = &ReaderUserSuggestionsParams, output = ReaderUserSuggestionsResponse)]
    Suggestions,
}

impl DerivedRequest for ReaderUsersRequest {
    fn namespace(&self) -> impl AsNamespace {
        // The newest version: `rest/v1.2` and `rest/v1.3` have no user profile, and no
        // suggestions (a 404). `rest/v1` answers suggestions as `rest/v1.1` does.
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

    #[rstest]
    fn suggestions(endpoint: ReaderUsersRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.suggestions(&ReaderUserSuggestionsParams::default()),
            "/users/suggest?",
        );
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.suggestions(&ReaderUserSuggestionsParams {
                site_id: Some(WpComSiteId(100000001)),
                site: Some("example.com".to_string()),
                filter: Some("ja".to_string()),
                image_size: Some(48),
                client: Some("an-app".to_string()),
            }),
            "/users/suggest?site_id=100000001&site=example.com&filter=ja&image_size=48&client=an-app",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ReaderUsersRequestEndpoint {
        ReaderUsersRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}
