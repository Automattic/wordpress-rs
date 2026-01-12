#[derive(wp_derive_request_builder::WpDerivedRequest)]
enum TestRequest {
    #[post(url = "/test", output = TestOutput, form_urlencoded = true)]
    Submit,
}

fn main() {}
