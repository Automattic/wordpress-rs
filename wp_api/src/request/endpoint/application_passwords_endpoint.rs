use wp_derive_request_builder::WpDerivedRequest;

use crate::application_passwords::{
    ApplicationPasswordCreateParams, ApplicationPasswordDeleteAllResponse,
    ApplicationPasswordDeleteResponse, ApplicationPasswordUpdateParams, ApplicationPasswordUuid,
    ApplicationPasswordWithEditContext, ApplicationPasswordWithEmbedContext,
    ApplicationPasswordWithViewContext, SparseApplicationPassword, SparseApplicationPasswordField,
};
use crate::users::UserId;

use super::{DerivedRequest, Namespace};

#[derive(WpDerivedRequest)]
#[Namespace("/wp/v2")]
#[SparseField(SparseApplicationPasswordField)]
enum ApplicationPasswordsRequest {
    #[post(url = "/users/<user_id>/application-passwords", params = &ApplicationPasswordCreateParams, output = ApplicationPasswordWithEditContext)]
    Create,
    #[delete(url = "/users/<user_id>/application-passwords/<application_password_uuid>", output = ApplicationPasswordDeleteResponse)]
    Delete,
    #[delete(url = "/users/<user_id>/application-passwords", output = ApplicationPasswordDeleteAllResponse)]
    DeleteAll,
    #[contextual_get(url = "/users/<user_id>/application-passwords", output = Vec<SparseApplicationPassword>, filter_by = SparseApplicationPasswordField)]
    List,
    #[contextual_get(url = "/users/<user_id>/application-passwords/<application_password_uuid>", output = SparseApplicationPassword, filter_by = SparseApplicationPasswordField)]
    Retrieve,
    #[contextual_get(url = "/users/<user_id>/application-passwords/introspect", output = SparseApplicationPassword, filter_by = SparseApplicationPasswordField)]
    RetrieveCurrent,
    #[post(url = "/users/<user_id>/application-passwords/<application_password_uuid>", params = &ApplicationPasswordUpdateParams, output = ApplicationPasswordWithEditContext)]
    Update,
}

impl DerivedRequest for ApplicationPasswordsRequest {
    fn namespace() -> Namespace {
        Namespace::WpV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::{
            tests::{fixture_api_base_url, validate_wp_v2_endpoint},
            ApiBaseUrl,
        },
        WpContext,
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn create_application_password(endpoint: ApplicationPasswordsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.create(&UserId(1)),
            "/users/1/application-passwords",
        );
    }

    #[rstest]
    fn delete_single_application_password(endpoint: ApplicationPasswordsRequestEndpoint) {
        validate_wp_v2_endpoint(
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
        validate_wp_v2_endpoint(
            endpoint.delete_all(&UserId(1)),
            "/users/1/application-passwords",
        );
    }

    #[rstest]
    fn list_application_passwords_with_edit_context(endpoint: ApplicationPasswordsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&UserId(2)),
            "/users/2/application-passwords?context=edit",
        );
    }

    #[rstest]
    fn list_application_passwords_with_embed_context(
        endpoint: ApplicationPasswordsRequestEndpoint,
    ) {
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(&UserId(71)),
            "/users/71/application-passwords?context=embed",
        );
    }

    #[rstest]
    fn list_application_passwords_with_view_context(endpoint: ApplicationPasswordsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&UserId(9999)),
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
        validate_wp_v2_endpoint(
            endpoint.filter_list(&UserId(2), context, fields),
            expected_path,
        );
    }

    #[rstest]
    fn retrieve_current_application_passwords_with_edit_context(
        endpoint: ApplicationPasswordsRequestEndpoint,
    ) {
        validate_wp_v2_endpoint(
            endpoint.retrieve_current_with_edit_context(&UserId(2)),
            "/users/2/application-passwords/introspect?context=edit",
        );
    }

    #[rstest]
    #[case(WpContext::Edit, &[SparseApplicationPasswordField::Uuid], "/users/2/application-passwords/introspect?context=edit&_fields=uuid")]
    #[case(WpContext::View, &[SparseApplicationPasswordField::Uuid, SparseApplicationPasswordField::Name], "/users/2/application-passwords/introspect?context=view&_fields=uuid%2Cname")]
    fn filter_retrieve_current_application_passwords(
        endpoint: ApplicationPasswordsRequestEndpoint,
        #[case] context: WpContext,
        #[case] fields: &[SparseApplicationPasswordField],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_current(&UserId(2), context, fields),
            expected_path,
        );
    }

    #[rstest]
    fn retrieve_application_passwords_with_embed_context(
        endpoint: ApplicationPasswordsRequestEndpoint,
    ) {
        validate_wp_v2_endpoint(
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
    #[case(WpContext::Edit, &[SparseApplicationPasswordField::Uuid], "/users/2/application-passwords/584a87d5-4f18-4c33-a315-4c05ed1fc485?context=edit&_fields=uuid")]
    #[case(WpContext::View, &[SparseApplicationPasswordField::Uuid, SparseApplicationPasswordField::Password], "/users/2/application-passwords/584a87d5-4f18-4c33-a315-4c05ed1fc485?context=view&_fields=uuid%2Cpassword")]
    fn filter_retrieve_application_passwords(
        endpoint: ApplicationPasswordsRequestEndpoint,
        #[case] context: WpContext,
        #[case] fields: &[SparseApplicationPasswordField],
        #[case] expected_path: &str,
    ) {
        let uuid = ApplicationPasswordUuid {
            uuid: "584a87d5-4f18-4c33-a315-4c05ed1fc485".to_string(),
        };
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve(&UserId(2), &uuid, context, fields),
            expected_path,
        );
    }

    #[rstest]
    fn update_application_password(endpoint: ApplicationPasswordsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.update(
                &UserId(2),
                &ApplicationPasswordUuid {
                    uuid: "584a87d5-4f18-4c33-a315-4c05ed1fc485".to_string(),
                },
            ),
            "/users/2/application-passwords/584a87d5-4f18-4c33-a315-4c05ed1fc485",
        );
    }

    #[fixture]
    fn endpoint(fixture_api_base_url: Arc<ApiBaseUrl>) -> ApplicationPasswordsRequestEndpoint {
        ApplicationPasswordsRequestEndpoint::new(fixture_api_base_url)
    }
}
