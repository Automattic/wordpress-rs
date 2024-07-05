#[derive(wp_derive_request_builder::WpDerivedRequest)]
#[Namespace("/wp/v2")]
#[SparseField(crate::SparseUserField)]
enum UsersRequest {
    #[contextual_get(params = &UserListParams, output = Vec<SparseUser>)]
    List,
}

fn main() {}
