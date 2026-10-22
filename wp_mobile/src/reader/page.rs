use std::sync::LazyLock;

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use sha2::{Digest, Sha256};

use super::assets::{reader_script, reader_stylesheet};
use super::embeds::policy_hosts;
use super::theme::{
    ReaderAppColors, ReaderFont, ReaderTheme, reader_page_colors_css, reader_page_theme_css,
    reader_page_typography_css,
};

/// What a Reader post page needs besides the post: see
/// [`ReaderPostContent::page_html`](super::ReaderPostContent::page_html).
#[derive(Debug, Clone, uniffi::Record)]
pub struct ReaderPageOptions {
    /// The reader's theme.
    pub theme: ReaderTheme,
    /// The app's own colours, which the System and Reader themes use.
    pub app_colors: ReaderAppColors,
    /// The site's language, like `fa` or `pt-br`, so the page's text runs in the direction it does
    /// on the site. The Reader API has it in `meta.data.site.locale`, for signed-in requests.
    /// Without it, the page runs in the direction most of the post's letters do. Parts of the post
    /// with their own `dir` keep it.
    #[uniffi(default = None)]
    pub language: Option<String>,
    /// Whether the page is for reading offline: it shows the post without the image sizes it
    /// would download, and loads no embed scripts.
    #[uniffi(default = false)]
    pub offline: bool,
    /// The typeface from the reader's settings. Without one, it's the platform's own.
    #[uniffi(default = None)]
    pub font: Option<ReaderFont>,
    /// How much bigger or smaller than 16px to make the text: the reader's size setting, times the
    /// system's text size. See [`reader_page_typography_css`](super::reader_page_typography_css).
    #[uniffi(default = 1.0)]
    pub text_scale: f64,
    /// CSS from WordPress.com, after the shared stylesheet, to fix it without waiting for an app
    /// release. The apps fetch `reader-mobile.css` for this today.
    #[uniffi(default = None)]
    pub server_css: Option<String>,
    /// The app's own CSS, after everything else.
    #[uniffi(default = None)]
    pub css: Option<String>,
}

/// The base URL to load a Reader post page with. Some embeds need a page on the web: YouTube's
/// player won't play on a page without one. Links within the post are already `#anchor` links, and
/// the cleanup resolves the post's relative URLs against its own address, so this doesn't change
/// where they go.
#[uniffi::export]
pub fn reader_page_base_url() -> String {
    "https://wordpress.com/reader".to_owned()
}

/// The page for a post: the post's HTML, with the theme's colours, the shared stylesheet and script,
/// and the scripts its embeds need. `text` is the post's text, for its direction.
/// `reader_js_scripts` are scripts the shared script loads itself, like gists' data.
///
/// Its Content Security Policy only lets those scripts run, so if the cleanup misses something
/// that would run a script, the web view still won't run it.
pub(super) fn page_html(
    html: &str,
    text: &str,
    scripts: &[String],
    reader_js_scripts: &[String],
    options: ReaderPageOptions,
) -> String {
    let language = options.language.as_deref().and_then(language_tag);
    let right_to_left = match &language {
        Some(language) => is_right_to_left(language),
        None => is_mostly_right_to_left(text),
    };
    let direction = if right_to_left { "rtl" } else { "ltr" };
    let html_attributes = match &language {
        Some(language) => format!(r#" lang="{language}" dir="{direction}""#),
        None => format!(r#" dir="{direction}""#),
    };
    let style = |css: Option<String>| {
        css.map(|css| format!("<style>{}</style>\n", style_text(&css)))
            .unwrap_or_default()
    };
    let typography = reader_page_typography_css(
        options.font.unwrap_or(ReaderFont::Sans),
        options.text_scale,
        options.theme,
    );
    let theme_css = style(Some(reader_page_theme_css(options.theme)).filter(|css| !css.is_empty()));
    let server_css = style(options.server_css);
    let app_css = style(options.css);
    let scripts: &[String] = if options.offline { &[] } else { scripts };
    let policy_scripts: Vec<String> = if options.offline {
        Vec::new()
    } else {
        scripts.iter().chain(reader_js_scripts).cloned().collect()
    };
    let policy = attribute(&content_security_policy(&policy_scripts));
    let scripts: String = scripts
        .iter()
        .map(|script| format!("<script async src=\"{}\"></script>\n", attribute(script)))
        .collect();

    format!(
        "<!DOCTYPE html>\n\
         <html{html_attributes}>\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta http-equiv=\"Content-Security-Policy\" content=\"{policy}\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <style>{colors}</style>\n\
         <style>{typography}</style>\n\
         <style>{stylesheet}</style>\n\
         {theme_css}\
         {server_css}\
         {app_css}\
         <script>{script}</script>\n\
         </head>\n\
         <body>\n\
         {html}\n\
         {scripts}\
         </body>\n\
         </html>\n",
        colors = reader_page_colors_css(options.theme, options.app_colors),
        stylesheet = reader_stylesheet(),
        script = reader_script(),
    )
}

/// The page's Content Security Policy. Scripts can only be the shared script, which the policy
/// knows by its hash, or come from the hosts of the embeds' scripts. So nothing in the post runs:
/// not inline scripts, not event handlers like `onclick`, and not `javascript:` URLs. Nor can the
/// post load plugins, change the page's base URL, or submit a form.
///
/// It doesn't cover styles, which the app's CSS and the theme add inline, or iframes, which run
/// on their own sites, under their own policies.
fn content_security_policy(scripts: &[String]) -> String {
    static SCRIPT_HASH: LazyLock<String> =
        LazyLock::new(|| BASE64.encode(Sha256::digest(reader_script().as_bytes())));
    let hosts: String = policy_hosts(scripts)
        .iter()
        .map(|host| format!(" https://{host}"))
        .collect();
    format!(
        "script-src 'sha256-{}'{hosts}; object-src 'none'; base-uri 'none'; form-action 'none'",
        *SCRIPT_HASH
    )
}

/// A language tag that's safe in the page, like `pt-BR` for WordPress's `pt_BR`, or `None` if
/// `language` isn't one.
fn language_tag(language: &str) -> Option<String> {
    let tag = language.trim().replace('_', "-");
    let is_tag = !tag.is_empty()
        && tag.len() <= 35
        && tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    is_tag.then_some(tag)
}

/// Whether a language is written right to left, from its tag's primary language. Every language
/// WordPress has a right-to-left locale for is here.
fn is_right_to_left(language: &str) -> bool {
    let primary = language
        .split('-')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(
        primary.as_str(),
        "ar" | "arc"
            | "arq"
            | "ary"
            | "azb"
            | "bcc"
            | "bgn"
            | "ckb"
            | "dv"
            | "fa"
            | "haz"
            | "he"
            | "iw"
            | "nqo"
            | "ps"
            | "sd"
            | "skr"
            | "snd"
            | "ug"
            | "ur"
            | "yi"
    )
}

/// Whether most of the letters in `text` are from scripts written right to left, like Arabic and
/// Hebrew. A post that quotes a few words of Hebrew still runs left to right.
fn is_mostly_right_to_left(text: &str) -> bool {
    let (right_to_left, left_to_right) =
        text.chars()
            .filter(|c| c.is_alphabetic())
            .fold((0usize, 0usize), |(rtl, ltr), c| {
                if is_right_to_left_letter(c) {
                    (rtl + 1, ltr)
                } else {
                    (rtl, ltr + 1)
                }
            });
    right_to_left > left_to_right
}

/// Whether a letter is from a script written right to left: Hebrew, Arabic, Syriac, Thaana, N'Ko
/// and the like, and their presentation forms. Unicode keeps these blocks for them.
fn is_right_to_left_letter(c: char) -> bool {
    matches!(
        c,
        '\u{0590}'..='\u{08FF}'
            | '\u{FB1D}'..='\u{FDFF}'
            | '\u{FE70}'..='\u{FEFF}'
            | '\u{10800}'..='\u{10FFF}'
            | '\u{1E800}'..='\u{1EFFF}'
    )
}

/// CSS for a `<style>` element: `</style` would end the element early.
fn style_text(css: &str) -> String {
    css.replace("</", "<\\/")
}

/// Text for a double-quoted HTML attribute.
fn attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

#[cfg(test)]
mod tests {
    use super::{BASE64, ReaderPageOptions};
    use crate::reader::{
        ReaderBrand, ReaderFont, ReaderPostContent, ReaderTheme, reader_default_app_colors,
        reader_page_colors_css, reader_page_theme_css, reader_page_typography_css, reader_script,
        reader_stylesheet,
    };
    use base64::Engine;
    use rstest::rstest;
    use sha2::{Digest, Sha256};

    fn options() -> ReaderPageOptions {
        ReaderPageOptions {
            theme: ReaderTheme::Sepia,
            app_colors: reader_default_app_colors(ReaderBrand::Wordpress),
            language: None,
            offline: false,
            font: None,
            text_scale: 1.0,
            server_css: None,
            css: None,
        }
    }

    #[test]
    fn puts_the_post_in_a_page_with_the_shared_styles_and_script() {
        let content = ReaderPostContent::new("<p>Hi</p>", None);
        let page = content.page_html(options());

        assert!(page.starts_with("<!DOCTYPE html>\n<html dir=\"ltr\">\n<head>\n"));
        let colors = reader_page_colors_css(options().theme, options().app_colors);
        let head_end = page.find("</head>").expect("the page has a head");
        for part in [colors.as_str(), &reader_stylesheet(), &reader_script()] {
            let position = page.find(part).expect("the page has every part");
            assert!(
                position < head_end,
                "everything but the post is in the head"
            );
        }
        // The theme's colours come first, so the stylesheet can use them.
        assert!(page.find(colors.as_str()) < page.find(&reader_stylesheet()));
        assert!(page.contains("<body>\n<p>Hi</p>\n</body>"));
    }

    #[test]
    fn adds_the_reader_themes_text_after_the_stylesheet() {
        let content = ReaderPostContent::new("<p>Hi</p>", None);
        let page = content.page_html(ReaderPageOptions {
            theme: ReaderTheme::Reader,
            ..options()
        });
        let text = reader_page_theme_css(ReaderTheme::Reader);
        let position = page.find(&text).expect("the page has the theme's text");
        // After the stylesheet, whose text size and line height it replaces.
        assert!(page.find(&reader_stylesheet()) < Some(position));

        // The other themes have nothing to add.
        assert_eq!(content.page_html(options()).matches("<style>").count(), 3);
    }

    #[test]
    fn loads_embed_scripts_after_the_post() {
        let content = ReaderPostContent::new(r#"<div class="fb-post"></div>"#, None);
        let page = content.page_html(options());
        assert!(page.contains(
            "<div class=\"fb-post\"></div>\n<script async src=\"https://connect.facebook.net/en_US/sdk.js#xfbml=1&amp;version=v2.8\"></script>\n</body>"
        ));
    }

    #[test]
    fn only_lets_the_shared_script_and_the_embeds_scripts_run() {
        let content = ReaderPostContent::new(r#"<div class="fb-post"></div>"#, None);
        let page = content.page_html(options());

        let hash = BASE64.encode(Sha256::digest(reader_script().as_bytes()));
        let policy = format!(
            "<meta http-equiv=\"Content-Security-Policy\" content=\"script-src 'sha256-{hash}' https://connect.facebook.net; object-src 'none'; base-uri 'none'; form-action 'none'\">"
        );
        let position = page.find(&policy).expect("the page has the policy");
        // A policy only covers what comes after it.
        assert!(page.find("<style>") > Some(position));
        assert!(page.find("<script") > Some(position));

        let offline = content.page_html(ReaderPageOptions {
            offline: true,
            ..options()
        });
        assert!(offline.contains(&format!("script-src 'sha256-{hash}'; object-src")));
    }

    #[test]
    fn makes_an_offline_page() {
        let content = ReaderPostContent::new(
            r#"<div class="fb-post"></div><img src="a.jpg" srcset="a-300.jpg 300w">"#,
            None,
        );
        let page = content.page_html(ReaderPageOptions {
            offline: true,
            ..options()
        });
        assert!(page.contains(&content.offline_html()));
        assert!(!page.contains("srcset"));
        assert!(!page.contains("<script async"));
    }

    #[rstest]
    #[case::right_to_left(Some("fa"), "<p>Hi</p>", r#"<html lang="fa" dir="rtl">"#)]
    #[case::right_to_left_with_a_region(
        Some("ar_EG"),
        "<p>Hi</p>",
        r#"<html lang="ar-EG" dir="rtl">"#
    )]
    // WordPress's right-to-left locales with a three-letter language.
    #[case::algerian_arabic(Some("arq"), "<p>Hi</p>", r#"<html lang="arq" dir="rtl">"#)]
    #[case::moroccan_arabic(Some("ary"), "<p>Hi</p>", r#"<html lang="ary" dir="rtl">"#)]
    #[case::south_azerbaijani(Some("azb"), "<p>Hi</p>", r#"<html lang="azb" dir="rtl">"#)]
    #[case::southern_balochi(Some("bcc"), "<p>Hi</p>", r#"<html lang="bcc" dir="rtl">"#)]
    #[case::western_balochi(Some("bgn"), "<p>Hi</p>", r#"<html lang="bgn" dir="rtl">"#)]
    #[case::hazaragi(Some("haz"), "<p>Hi</p>", r#"<html lang="haz" dir="rtl">"#)]
    #[case::nko(Some("nqo"), "<p>Hi</p>", r#"<html lang="nqo" dir="rtl">"#)]
    #[case::saraiki(Some("skr"), "<p>Hi</p>", r#"<html lang="skr" dir="rtl">"#)]
    #[case::sindhi(Some("snd"), "<p>Hi</p>", r#"<html lang="snd" dir="rtl">"#)]
    #[case::left_to_right(Some("pt_BR"), "<p>Hi</p>", r#"<html lang="pt-BR" dir="ltr">"#)]
    // Azerbaijani is written in Latin letters; only South Azerbaijani is right to left.
    #[case::azerbaijani(Some("az"), "<p>Hi</p>", r#"<html lang="az" dir="ltr">"#)]
    // WordPress themes take the direction from the site's language, so the page does too.
    #[case::the_sites_language_over_the_posts(
        Some("en"),
        "<p>مرحبا بالعالم</p>",
        r#"<html lang="en" dir="ltr">"#
    )]
    #[case::unknown(None, "<p>Hi</p>", r#"<html dir="ltr">"#)]
    #[case::unknown_in_arabic(None, "<p>مرحبا بالعالم، من WordPress</p>", r#"<html dir="rtl">"#)]
    #[case::unknown_in_hebrew(None, "<p>שלום עולם</p>", r#"<html dir="rtl">"#)]
    #[case::unknown_quoting_a_little_hebrew(
        None,
        "<p>They said <q>שלום</q> and waved goodbye.</p>",
        r#"<html dir="ltr">"#
    )]
    #[case::unknown_without_letters(None, "<p>2026 — 🙂</p>", r#"<html dir="ltr">"#)]
    #[case::not_a_language(Some(r#"en" onload="alert(1)"#), "<p>Hi</p>", r#"<html dir="ltr">"#)]
    fn sets_the_language_and_direction(
        #[case] language: Option<&str>,
        #[case] post: &str,
        #[case] expected: &str,
    ) {
        let content = ReaderPostContent::new(post, None);
        let page = content.page_html(ReaderPageOptions {
            language: language.map(str::to_owned),
            ..options()
        });
        assert!(page.contains(&format!("{expected}\n<head>")), "{page}");
    }

    #[test]
    fn adds_the_apps_css_after_the_shared_stylesheet() {
        let content = ReaderPostContent::new("<p>Hi</p>", None);
        let page = content.page_html(ReaderPageOptions {
            css: Some(
                "body { font-family: serif; } /* </style><script>alert(1)</script> */".to_owned(),
            ),
            ..options()
        });
        let app_css = page
            .find("body { font-family: serif; }")
            .expect("the page has the app's CSS");
        assert!(page.find(&reader_stylesheet()) < Some(app_css));
        // It can't end its `<style>` early.
        assert!(!page.contains("</style><script>alert(1)"));
    }

    #[test]
    fn sets_the_readers_font_and_text_size() {
        let content = ReaderPostContent::new("<p>Hi</p>", None);
        let page = content.page_html(ReaderPageOptions {
            font: Some(ReaderFont::Serif),
            text_scale: 1.25,
            ..options()
        });
        let typography = reader_page_typography_css(ReaderFont::Serif, 1.25, options().theme);
        let position = page
            .find(&typography)
            .expect("the page sets the typography");
        // Before the stylesheet, which reads it.
        assert!(Some(position) < page.find(&reader_stylesheet()));
    }

    #[test]
    fn adds_the_server_css_between_the_shared_stylesheet_and_the_apps() {
        let content = ReaderPostContent::new("<p>Hi</p>", None);
        let page = content.page_html(ReaderPageOptions {
            server_css: Some("p { color: red; } </style><script>alert(1)</script>".to_owned()),
            css: Some("p { color: blue; }".to_owned()),
            ..options()
        });
        let server = page.find("p { color: red; }");
        assert!(page.find(&reader_stylesheet()) < server);
        assert!(server < page.find("p { color: blue; }"));
        assert!(!page.contains("</style><script>alert(1)"));
    }
}
