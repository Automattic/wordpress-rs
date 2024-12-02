use rstest::*;
use std::collections::HashMap;
use std::path::PathBuf;
use wp_org_api::plugin_directory::*;

#[fixture]
fn plugins_dir() -> PathBuf {
    std::fs::canonicalize("../target/wordpress-org-plugin-directory").unwrap()
}

#[fixture]
fn plugin_info_files(plugins_dir: PathBuf) -> Vec<PathBuf> {
    println!(
        "Reading plugin information files from {:?}...",
        &plugins_dir
    );

    let mut files = vec![];
    for entry in std::fs::read_dir(plugins_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file()
            && entry.path().extension().and_then(|f| f.to_str()) == Some("json")
        {
            files.push(entry.path());
        }
    }
    files
}

fn parse_plugin(slug: &str) -> PluginInformation {
    let file = plugins_dir().join(format!("{}.json", slug));
    let content = std::fs::read_to_string(file).unwrap();
    let result = serde_json::from_str::<PluginInformation>(&content);

    assert!(
        result.is_ok(),
        "Failed to parse plugin {:?}: {:?}",
        slug,
        result.err()
    );

    result.unwrap()
}

#[rstest]
fn parse_plugin_info(plugin_info_files: Vec<PathBuf>) {
    let results: HashMap<&PathBuf, _> =
        plugin_info_files
            .iter()
            .fold(HashMap::new(), |mut results, file| {
                let info = std::fs::read_to_string(file).unwrap();
                let result = serde_json::from_str::<PluginInformation>(&info);
                results.insert(file, result);
                results
            });

    let success: Vec<_> = results
        .iter()
        .filter(|(_, result)| result.is_ok())
        .collect();

    let failed: Vec<_> = results
        .iter()
        .filter(|(_, result)| result.is_err())
        .map(|(file, _)| file)
        .collect();
    if !failed.is_empty() {
        println!("Failed to parse the following files:");
        for file in &failed {
            println!("- {:?}", file.file_name().unwrap());
        }
    }

    assert_eq!(
        success.len(),
        results.len(),
        "{} out of {} files parsed successfully",
        success.len(),
        results.len()
    );
    assert!(
        failed.is_empty(),
        "Failed to parse {} out of {} files",
        failed.len(),
        results.len()
    );
}

#[rstest]
#[case("jetpack", "https://profiles.wordpress.org/automattic/")]
#[case("appmysite", "https://profiles.wordpress.org/appmysite/")]
#[case("superlinks", "")]
fn test_property_author_profile(#[case] slug: &str, #[case] value: &str) {
    let result = parse_plugin(slug);
    assert_eq!(result.author_profile, value);
}

#[rstest]
#[case("jetpack", "automattic", "https://profiles.wordpress.org/automattic/")]
#[case("appmysite", "appmysite", "https://profiles.wordpress.org/appmysite/")]
fn test_property_contributors_profile(
    #[case] slug: &str,
    #[case] username: &str,
    #[case] profile: &str,
) {
    let result = parse_plugin(slug);
    let contributors = result.contributors;
    assert_eq!(
        contributors.get(username).map(|f| f.profile.as_str()),
        Some(profile)
    );
}

#[rstest]
#[case("superlinks")]
fn test_property_no_contributors(#[case] slug: &str) {
    let result = parse_plugin(slug);
    assert!(result.contributors.is_empty());
}

#[rstest]
#[case("timeline-express-no-icons-add-on", "")]
#[case("appmysite", "6.4")]
#[case("superlinks", "2.5")]
fn test_property_requires(#[case] slug: &str, #[case] value: &str) {
    let result = parse_plugin(slug);
    assert_eq!(result.requires, value);
}

#[rstest]
#[case("jetpack", "6.7.1")]
#[case("add-rss", "")]
fn test_property_tested(#[case] slug: &str, #[case] value: &str) {
    let result = parse_plugin(slug);
    assert_eq!(result.tested, value);
}

#[rstest]
#[case("about-author", "")]
#[case("accessibility-toolbar", "7.4")]
fn test_property_requires_php(#[case] slug: &str, #[case] value: &str) {
    let result = parse_plugin(slug);
    assert_eq!(result.requires_php, value);
}

#[rstest]
#[case("accordion-archive-widget", "")]
#[case("abc-pricing-table", "commercial")]
fn test_property_business_model(#[case] slug: &str, #[case] value: &str) {
    let result = parse_plugin(slug);
    assert_eq!(result.business_model, value);
}

#[rstest]
fn test_property_empty_upgrade_notice(#[values("1-click-close-store", "2em")] slug: &str) {
    let result = parse_plugin(slug);
    assert!(result.upgrade_notice.is_empty());
}

#[rstest]
fn test_property_nonempty_upgrade_notice(
    #[values("ab-wp-security", "absolute-addons")] slug: &str,
) {
    let result = parse_plugin(slug);
    assert!(!result.upgrade_notice.is_empty());
}

#[rstest]
fn test_property_empty_tags(#[values("acf-rest", "add-rss")] slug: &str) {
    let result = parse_plugin(slug);
    assert!(result.tags.is_empty());
}

#[rstest]
fn test_property_nonempty_tags(#[values("appbanners", "seo-assistant")] slug: &str) {
    let result = parse_plugin(slug);
    assert!(!result.tags.is_empty());
}

#[rstest]
fn test_property_empty_versions(#[values("mos-faqs", "adjustly-collapse")] slug: &str) {
    let result = parse_plugin(slug);
    assert!(result.versions.is_empty());
}

#[rstest]
fn test_property_nonempty_versions(#[values("abcsubmit", "acf-views")] slug: &str) {
    let result = parse_plugin(slug);
    assert!(!result.versions.is_empty());
}

#[rstest]
#[case(
    "appmysite",
    "https://ps.w.org/appmysite/assets/banner-772x250.png?rev=2829272",
    "https://ps.w.org/appmysite/assets/banner-1544x500.png?rev=2829272"
)]
#[case(
    "jetpack",
    "https://ps.w.org/jetpack/assets/banner-772x250.png?rev=2653649",
    "https://ps.w.org/jetpack/assets/banner-1544x500.png?rev=2653649"
)]
#[case(
    "1-click-migration",
    "https://ps.w.org/1-click-migration/assets/banner-772x250.png?rev=2333853",
    ""
)]
fn test_property_banners(#[case] slug: &str, #[case] low: String, #[case] high: String) {
    let result = parse_plugin(slug);
    let expected = Banners { low, high };
    let banners = result.banners;
    assert_eq!(banners, expected);
}

#[rstest]
#[case(
    "contact-form-7",
    Some("https://ps.w.org/contact-form-7/assets/icon.svg?rev=2339255"),
    None,
    Some("https://ps.w.org/contact-form-7/assets/icon.svg?rev=2339255"),
    None
)]
#[case(
    "adminimize",
    None,
    None,
    None,
    Some("https://s.w.org/plugins/geopattern-icon/adminimize_000000.svg")
)]
fn test_property_icons(
    #[case] slug: &str,
    #[case] low: Option<&str>,
    #[case] high: Option<&str>,
    #[case] svg: Option<&str>,
    #[case] default: Option<&str>,
) {
    let result = parse_plugin(slug);
    let icons = result.icons;
    assert!(icons.is_some());

    let icons = icons.unwrap();
    assert_eq!(icons.low.as_deref(), low);
    assert_eq!(icons.high.as_deref(), high);
    assert_eq!(icons.svg.as_deref(), svg);
    assert_eq!(icons.default.as_deref(), default);
}
