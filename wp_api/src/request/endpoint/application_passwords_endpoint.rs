use wp_derive_request_builder::WpDerivedRequest;

use crate::application_passwords::{
    ApplicationPasswordCreateParams, ApplicationPasswordDeleteAllResponse,
    ApplicationPasswordDeleteResponse, ApplicationPasswordUuid, ApplicationPasswordWithEditContext,
    ApplicationPasswordWithEmbedContext, ApplicationPasswordWithViewContext,
    SparseApplicationPassword, SparseApplicationPasswordField,
};
use crate::users::UserId;

#[derive(WpDerivedRequest)]
#[SparseField(SparseApplicationPasswordField)]
enum ApplicationPasswordsRequest {
    #[post(url = "/users/<user_id>/application-passwords", params = &ApplicationPasswordCreateParams, output = ApplicationPasswordWithEditContext)]
    Create,
    #[delete(url = "/users/<user_id>/application-passwords/<application_password_uuid>", output = ApplicationPasswordDeleteResponse)]
    Delete,
    #[delete(url = "/users/<user_id>/application-passwords", output = ApplicationPasswordDeleteAllResponse)]
    DeleteAll,
    #[contextual_get(url = "/users/<user_id>/application-passwords", output = Vec<SparseApplicationPassword>)]
    List,
    #[contextual_get(url = "/users/<user_id>/application-passwords/<application_password_uuid>", output = SparseApplicationPassword)]
    Retrieve,
    #[contextual_get(url = "/users/<user_id>/application-passwords/introspect", output = SparseApplicationPassword)]
    RetrieveCurrent,
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
    fn create_application_password(endpoint: ApplicationPasswordsRequestEndpoint) {
        validate_endpoint(
            endpoint.create(&UserId(1)),
            "/users/1/application-passwords",
        );
    }

    #[rstest]
    fn delete_single_application_password(endpoint: ApplicationPasswordsRequestEndpoint) {
        validate_endpoint(
            endpoint.delete(
                &UserId(2),
                &ApplicationPasswordUuid {
                    uuid: "584a87d5-4f18-4c33-a315-4c05ed1fc485".to_string(),
                },
            ),
            "/users/2/application-passwords/584a87d5-4f18-4c33-a315-4c05ed1fc485",
        );
    }

    #[rstest]
    fn delete_all_application_passwords(endpoint: ApplicationPasswordsRequestEndpoint) {
        validate_endpoint(
            endpoint.delete_all(&UserId(1)),
            "/users/1/application-passwords",
        );
    }

    #[rstest]
    fn list_application_passwords_with_edit_context(endpoint: ApplicationPasswordsRequestEndpoint) {
        validate_endpoint(
            endpoint.list_with_edit_context(&UserId(2)),
            "/users/2/application-passwords?context=edit",
        );
    }

    #[rstest]
    fn list_application_passwords_with_embed_context(
        endpoint: ApplicationPasswordsRequestEndpoint,
    ) {
        validate_endpoint(
            endpoint.list_with_embed_context(&UserId(71)),
            "/users/71/application-passwords?context=embed",
        );
    }

    #[rstest]
    fn list_application_passwords_with_view_context(endpoint: ApplicationPasswordsRequestEndpoint) {
        validate_endpoint(
            endpoint.list_with_view_context(&UserId(9999)),
            "/users/9999/application-passwords?context=view",
        );
    }

    #[rstest]
    fn retrieve_current_application_passwords_with_edit_context(
        endpoint: ApplicationPasswordsRequestEndpoint,
    ) {
        validate_endpoint(
            endpoint.retrieve_current_with_edit_context(&UserId(2)),
            "/users/2/application-passwords/introspect?context=edit",
        );
    }

    #[rstest]
    fn retrieve_application_passwords_with_embed_context(
        endpoint: ApplicationPasswordsRequestEndpoint,
    ) {
        validate_endpoint(
            endpoint.retrieve_with_embed_context(
                &UserId(2),
                &ApplicationPasswordUuid {
                    uuid: "584a87d5-4f18-4c33-a315-4c05ed1fc485".to_string(),
                },
            ),
            "/users/2/application-passwords/584a87d5-4f18-4c33-a315-4c05ed1fc485?context=embed",
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
            endpoint.filter_list(&UserId(2), context, fields),
            expected_path,
        );
    }

    #[fixture]
    fn endpoint(fixture_api_base_url: Arc<ApiBaseUrl>) -> ApplicationPasswordsRequestEndpoint {
        ApplicationPasswordsRequestEndpoint::new(fixture_api_base_url)
    }
}
