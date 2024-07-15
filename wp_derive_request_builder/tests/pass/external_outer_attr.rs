#[derive(wp_derive_request_builder::WpDerivedRequest, serde::Serialize)]
#[serde(deny_unknown_fields)]
enum UsersRequest {
    #[contextual_get(url = "/users", params = &UserListParams, output = Vec<SparseUser>)]
    List,
}

fn main() {}
