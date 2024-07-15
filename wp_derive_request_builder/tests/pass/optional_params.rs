#[derive(wp_derive_request_builder::WpDerivedRequest)]
enum UsersRequest {
    #[contextual_get(url = "/users", output = Vec<SparseUser>)]
    List,
}

fn main() {}
