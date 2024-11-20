#[derive(wp_derive_request_builder::WpDerivedRequest)]
enum UsersRequest {
    #[post(url = "/users", output = Vec<SparseUser>)]
    List,
}

fn main() {}
