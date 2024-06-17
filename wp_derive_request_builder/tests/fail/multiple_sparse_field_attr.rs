#[derive(wp_derive_request_builder::WpDerivedRequest)]
#[SparseField(crate::SparseUserField)]
#[SparseField(crate::SparsePluginField)]
enum UsersRequest {
    #[contextual_get(url = "/users", params = &UserListParams, output = Vec<SparseUser>)]
    List,
}

fn main() {}
