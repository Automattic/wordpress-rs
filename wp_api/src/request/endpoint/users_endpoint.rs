use crate::{
    SparseField, SparseUserFieldWithEditContext, SparseUserFieldWithEmbedContext,
    SparseUserFieldWithViewContext, UserCreateParams, UserDeleteParams, UserDeleteResponse, UserId,
    UserListParams, UserUpdateParams, UserWithEditContext,
};
use wp_derive_request_builder::WpDerivedRequest;

use super::{DerivedRequest, Namespace};

#[derive(WpDerivedRequest)]
enum UsersRequest {
    #[contextual_get(url = "/users", params = &UserListParams, output = Vec<crate::SparseUser>, filter_by = crate::SparseUserField)]
    List,
    #[post(url = "/users", params = &UserCreateParams, output = UserWithEditContext)]
    Create,
    #[delete(url = "/users/<user_id>", params = &UserDeleteParams, output = UserDeleteResponse)]
    Delete,
    #[delete(url = "/users/me", params = &UserDeleteParams, output = UserDeleteResponse)]
    DeleteMe,
    #[contextual_get(url = "/users/<user_id>", output = crate::SparseUser, filter_by = crate::SparseUserField)]
    Retrieve,
    #[contextual_get(url = "/users/me", output = crate::SparseUser, filter_by = crate::SparseUserField)]
    RetrieveMe,
    #[post(url = "/users/<user_id>", params = &UserUpdateParams, output = UserWithEditContext)]
    Update,
    #[post(url = "/users/me", params = &UserUpdateParams, output = UserWithEditContext)]
    UpdateMe,
}

impl DerivedRequest for UsersRequest {
    fn namespace() -> Namespace {
        Namespace::WpV2
    }
}

super::macros::default_sparse_field_implementation_from_field_name!(SparseUserFieldWithEditContext);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparseUserFieldWithEmbedContext
);
super::macros::default_sparse_field_implementation_from_field_name!(SparseUserFieldWithViewContext);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::{
            tests::{fixture_api_base_url, validate_wp_v2_endpoint},
            ApiBaseUrl,
        },
        WpApiParamUsersHasPublishedPosts,
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn create_user(endpoint: UsersRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.create(), "/users");
    }

    #[rstest]
    fn delete_user(endpoint: UsersRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.delete(
                &UserId(54),
                &UserDeleteParams {
                    reassign: UserId(98),
                },
            ),
            "/users/54?force=true&reassign=98",
        );
    }

    #[rstest]
    fn delete_current_user(endpoint: UsersRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.delete_me(&UserDeleteParams {
                reassign: UserId(98),
            }),
            "/users/me?force=true&reassign=98",
        );
    }

    #[rstest]
    fn list_users(endpoint: UsersRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&UserListParams::default()),
            "/users?context=edit",
        );
    }

    #[rstest]
    fn list_users_default_params_empty_fields(endpoint: UsersRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&UserListParams::default()),
            "/users?context=edit",
        );
    }

    #[rstest]
    fn list_users_with_params(endpoint: UsersRequestEndpoint) {
        let params = UserListParams {
            page: Some(2),
            per_page: Some(60),
            search: Some("foo".to_string()),
            exclude: Vec::new(),
            include: Vec::new(),
            offset: None,
            order: None,
            orderby: None,
            slug: vec!["bar".to_string(), "baz".to_string()],
            roles: Vec::new(),
            capabilities: Vec::new(),
            who: None,
            has_published_posts: Some(WpApiParamUsersHasPublishedPosts::True),
        };
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&params),
            "/users?context=edit&page=2&per_page=60&search=foo&slug=bar%2Cbaz&has_published_posts=true",
        );
    }

    #[rstest]
    fn filter_list_users_default_params_empty_fields(endpoint: UsersRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(&UserListParams::default(), &[]),
            "/users?context=edit&_fields=",
        );
    }

    #[rstest]
    fn filter_list_users_with_params(endpoint: UsersRequestEndpoint) {
        let params = UserListParams {
            page: Some(2),
            per_page: Some(60),
            search: Some("foo".to_string()),
            exclude: Vec::new(),
            include: Vec::new(),
            offset: None,
            order: None,
            orderby: None,
            slug: vec!["bar".to_string(), "baz".to_string()],
            roles: Vec::new(),
            capabilities: Vec::new(),
            who: None,
            has_published_posts: Some(WpApiParamUsersHasPublishedPosts::PostTypes(vec![
                "post".to_string(),
                "page".to_string(),
            ])),
        };
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(&params, &[SparseUserFieldWithEditContext::Name, SparseUserFieldWithEditContext::Email]),
            "/users?context=edit&page=2&per_page=60&search=foo&slug=bar%2Cbaz&has_published_posts=post%2Cpage&_fields=name%2Cemail",
        );
    }

    #[rstest]
    fn retrieve_user(endpoint: UsersRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&UserId(98)),
            "/users/98?context=view",
        );
    }

    #[rstest]
    fn filter_retrieve_user(endpoint: UsersRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(
                &UserId(98),
                &[
                    SparseUserFieldWithViewContext::Name,
                    SparseUserFieldWithViewContext::Url,
                ],
            ),
            "/users/98?context=view&_fields=name%2Curl",
        );
    }

    #[rstest]
    fn retrieve_current_user(endpoint: UsersRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.retrieve_me_with_embed_context(),
            "/users/me?context=embed",
        );
    }

    #[rstest]
    fn filter_retrieve_current_user(endpoint: UsersRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_me_with_embed_context(&[
                SparseUserFieldWithEmbedContext::Link,
                SparseUserFieldWithEmbedContext::Url,
            ]),
            "/users/me?context=embed&_fields=link%2Curl",
        );
    }

    #[rstest]
    fn update_user(endpoint: UsersRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.update(&UserId(98)), "/users/98");
    }

    #[rstest]
    fn update_current_user(endpoint: UsersRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.update_me(), "/users/me");
    }

    #[fixture]
    fn endpoint(fixture_api_base_url: Arc<ApiBaseUrl>) -> UsersRequestEndpoint {
        UsersRequestEndpoint::new(fixture_api_base_url)
    }
}
