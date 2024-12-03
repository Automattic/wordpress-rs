#![allow(unused_macros)]

// Run replace-test-parameters.sh to replace the parameters defined in this file
// with the full plugin directory on wordpress.org.
//
// Please note: build time will increase significantly after running the script.
//
// IMPORTANT: DO NOT COMMIT THE CHANGES TO THIS FILE AFTER RUNNING THE SCRIPT.

#[rstest_reuse::template]
#[rstest::rstest]
#[case("https://api.wordpress.org/plugins/info/1.2/?action=query_plugins&request%5Bpage%5D=1&request%5Bper_page%5D=200")]
#[case("https://api.wordpress.org/plugins/info/1.2/?action=query_plugins&request%5Bpage%5D=2&request%5Bper_page%5D=200")]
#[case("https://api.wordpress.org/plugins/info/1.2/?action=query_plugins&request%5Bpage%5D=273&request%5Bper_page%5D=200")]
#[case("https://api.wordpress.org/plugins/info/1.2/?action=query_plugins&request%5Bpage%5D=274&request%5Bper_page%5D=200")]
pub fn query_plugins_api_url(#[case] url: &str) {}

#[rstest_reuse::template]
#[rstest::rstest]
#[case("jetpack")]
pub fn plugin_information_slug_1(#[case] slug: &str) {}

#[rstest_reuse::template]
#[rstest::rstest]
#[case("woocommerce")]
pub fn plugin_information_slug_2(#[case] slug: &str) {}
