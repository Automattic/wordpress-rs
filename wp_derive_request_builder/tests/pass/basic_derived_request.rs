#[derive(wp_derive_request_builder::WpDerivedRequest)]
#[SparseField(crate::SparseUserField)]
enum UsersRequest {
    #[contextual_get(url = "/users", params = &UserListParams, output = Vec<SparseUser>)]
    List,
}

fn main() {}
