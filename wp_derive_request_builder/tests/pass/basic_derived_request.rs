#[derive(wp_derive_request_builder::WpDerivedRequest)]
#[SparseField(crate::SparseUserField)]
#[Namespace("/wp/v2")]
enum UsersRequest {
    #[contextual_get(url = "/users", params = &UserListParams, output = Vec<SparseUser>)]
    List,
}

fn main() {}
