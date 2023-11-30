fn main() {
    uniffi::generate_scaffolding("src/wordpress_api.udl").unwrap();
}
