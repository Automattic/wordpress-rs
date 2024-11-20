#[derive(wp_derive_request_builder::WpDerivedRequest)]
enum UsersRequest {
    #[contextual_get(url = "/users", params = &UserListParams, output = Vec<SparseUser>, filter_by = SparseUserField)]
    List,
    #[post(url = "/users/<user_id>", params = &UserUpdateParams, output = UserWithEditContext, content_disposition = &UserContentDisposition)]
    Update,
}

fn main() {}
