// On Apple platforms, the `os_log` wrapper the logger calls (see `src/os_log.rs`) is normally
// compiled by the Swift package, which the Makefile signals with `WP_API_OS_LOG_FROM_SWIFT_PACKAGE`.
// Other Apple builds, such as `cargo test` or the CLI on a Mac, link their own copy from here.
fn main() {
    println!("cargo:rerun-if-env-changed=WP_API_OS_LOG_FROM_SWIFT_PACKAGE");
    let source = "../native/swift/Sources/wordpress-api-os-log/wp_api_os_log.c";
    println!("cargo:rerun-if-changed={source}");
    println!("cargo:rerun-if-changed=../native/swift/Sources/wordpress-api-os-log/include");

    let is_apple = std::env::var("CARGO_CFG_TARGET_VENDOR").is_ok_and(|v| v == "apple");
    if is_apple && std::env::var_os("WP_API_OS_LOG_FROM_SWIFT_PACKAGE").is_none() {
        cc::Build::new()
            .file(source)
            .include("../native/swift/Sources/wordpress-api-os-log/include")
            .compile("wp_api_os_log");
    }
}
