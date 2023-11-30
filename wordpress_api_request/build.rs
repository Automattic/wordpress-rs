fn main() {
    uniffi::generate_scaffolding("src/wordpress_api_request.udl").unwrap();
}
