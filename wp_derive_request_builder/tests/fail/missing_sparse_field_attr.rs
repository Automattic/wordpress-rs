#[derive(wp_derive_request_builder::WpDerivedRequest)]
#[Namespace("/wp/v2")]
enum UsersRequest {
    #[contextual_get(url = "/users", output = std::vec::Vec<SparseUser>)]
    List,
}

fn main() {}
