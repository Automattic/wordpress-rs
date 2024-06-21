use wp_derive_request_builder::WpDerivedRequest;

use crate::application_passwords::SparseApplicationPasswordField;
use crate::users::UserId;

#[derive(WpDerivedRequest)]
#[SparseField(SparseApplicationPasswordField)]
enum ApplicationPasswordsRequest {
    #[contextual_get(url = "/users/<user_id>/application-passwords", output = Vec<crate::application_passwords::SparseApplicationPassword>)]
    List,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::{
            tests::{fixture_api_base_url, validate_endpoint},
            ApiBaseUrl,
        },
        WpContext,
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn list_application_passwords_with_edit_context(endpoint: ApplicationPasswordsRequestEndpoint) {
        validate_endpoint(
            endpoint.list_with_edit_context(UserId(2)),
            "/users/2/application-passwords?context=edit",
        );
    }

    #[rstest]
    fn list_application_passwords_with_embed_context(
        endpoint: ApplicationPasswordsRequestEndpoint,
    ) {
        validate_endpoint(
            endpoint.list_with_embed_context(UserId(71)),
            "/users/71/application-passwords?context=embed",
        );
    }

    #[rstest]
    fn list_application_passwords_with_view_context(endpoint: ApplicationPasswordsRequestEndpoint) {
        validate_endpoint(
            endpoint.list_with_view_context(UserId(9999)),
            "/users/9999/application-passwords?context=view",
        );
    }

    #[rstest]
    #[case(WpContext::Edit, &[SparseApplicationPasswordField::Uuid], "/users/2/application-passwords?context=edit&_fields=uuid")]
    #[case(WpContext::View, &[SparseApplicationPasswordField::Uuid, SparseApplicationPasswordField::Name], "/users/2/application-passwords?context=view&_fields=uuid%2Cname")]
    fn filter_list_application_passwords(
        endpoint: ApplicationPasswordsRequestEndpoint,
        #[case] context: WpContext,
        #[case] fields: &[SparseApplicationPasswordField],
        #[case] expected_path: &str,
    ) {
        validate_endpoint(
            endpoint.filter_list(UserId(2), context, fields),
            expected_path,
        );
    }

    #[fixture]
    fn endpoint(fixture_api_base_url: Arc<ApiBaseUrl>) -> ApplicationPasswordsRequestEndpoint {
        ApplicationPasswordsRequestEndpoint::new(fixture_api_base_url)
    }
}
