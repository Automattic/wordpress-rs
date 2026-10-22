//! `/reader`: shows a post from the WordPress.com Reader the way the apps will, in any theme.
//!
//! It fetches the post from the Reader API, cleans it up with `wp_mobile::reader`, and puts it in
//! the page the apps' web views show, with `ReaderPostContent::page_html`. That page's Content
//! Security Policy only runs the scripts the apps would.

use rocket::{
    State,
    http::Header,
    response::{Redirect, Responder},
};
use rocket_dyn_templates::{Template, context};
use serde_json::{Value, json};
use std::sync::Arc;
use url::Url;
use wp_mobile::reader::{
    ReaderColor, ReaderFixedTheme, ReaderFont, ReaderPageColors, ReaderPageOptions,
    ReaderPostAnchorKind, ReaderPostContent, ReaderThemeAppearance, ReaderThemeColors,
    reader_fixed_theme_from_id, reader_fixed_themes,
};

const API: &str = "https://public-api.wordpress.com/rest";

/// The themes the reader can pick, by the name the tool's URLs use, and their names: the tool's
/// Reader theme, the System theme, and the themes with fixed colours that `wp_mobile` lists.
fn themes() -> Vec<(String, &'static str)> {
    [
        ("reader".to_owned(), "Reader"),
        ("system".to_owned(), "System"),
    ]
    .into_iter()
    .chain(
        reader_fixed_themes()
            .into_iter()
            .map(|theme| (theme.id(), fixed_theme_name(theme))),
    )
    .collect()
}

/// A fixed theme's name. `wp_mobile` has none, since the apps translate their own.
fn fixed_theme_name(theme: ReaderFixedTheme) -> &'static str {
    match theme {
        ReaderFixedTheme::Soft => "Soft",
        ReaderFixedTheme::Sepia => "Sepia",
        ReaderFixedTheme::Evening => "Evening",
        ReaderFixedTheme::Oled => "OLED",
        ReaderFixedTheme::Hacker => "Hacker",
        ReaderFixedTheme::Candy => "Candy",
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![index, site, post, page]
}

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("wordpress-rs reader preview")
        .build()
        .expect("the HTTP client can be built")
}

/// The form, or where a site or post URL leads.
#[get("/reader?<url>")]
async fn index(url: Option<&str>, client: &State<reqwest::Client>) -> Result<Redirect, Template> {
    let Some(input) = url.map(str::trim).filter(|input| !input.is_empty()) else {
        return Err(Template::render("reader/index", context! {}));
    };
    match resolve(client, input).await {
        Ok(path) => Ok(Redirect::to(path)),
        Err(error) => Err(Template::render(
            "reader/index",
            context! { url: input, error },
        )),
    }
}

/// A site's recent posts.
#[get("/reader/<site>")]
async fn site(site: &str, client: &State<reqwest::Client>) -> Template {
    let url = format!("{API}/v1.2/read/sites/{site}/posts?number=30&fields=ID,title,URL,date");
    match get_json(client, &url).await {
        Ok(json) => {
            let posts: Vec<Value> = json["posts"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|post| {
                    json!({
                        "id": post["ID"],
                        "title": text(post["title"].as_str().unwrap_or_default()),
                        "url": post["URL"],
                        "date": post["date"].as_str().unwrap_or_default().get(..10),
                    })
                })
                .collect();
            Template::render("reader/site", context! { site, posts })
        }
        Err(error) => Template::render("reader/index", context! { url: site, error }),
    }
}

/// The typefaces from the reader's settings.
const FONTS: &[(&str, &str)] = &[("sans", "Sans"), ("serif", "Serif"), ("mono", "Mono")];

/// The iOS app's text sizes, as scales of the normal size.
const SIZES: &[f64] = &[0.75, 0.9, 1.0, 1.15, 1.25, 1.4, 1.6];

/// The appearances the reader can force, as the apps do by setting their web view's appearance.
/// `auto` follows this computer's.
const APPEARANCES: &[(&str, &str)] = &[("auto", "Auto"), ("light", "Light"), ("dark", "Dark")];

/// The apps' brands. The Reader and System themes take their link colour from the app's brand, and
/// the themes with fixed colours don't.
const BRANDS: &[(&str, &str)] = &[("wordpress", "WordPress"), ("jetpack", "Jetpack")];

/// A post, in one theme or all of them, with what the shared code found in it.
#[get(
    "/reader/<site>/<post>?<theme>&<brand>&<appearance>&<font>&<size>&<width>&<offline>&<photon>&<lang>"
)]
#[allow(clippy::too_many_arguments)]
async fn post(
    site: &str,
    post: u64,
    theme: Option<&str>,
    brand: Option<&str>,
    appearance: Option<&str>,
    font: Option<&str>,
    size: Option<f64>,
    width: Option<u32>,
    offline: Option<bool>,
    photon: Option<bool>,
    lang: Option<&str>,
    client: &State<reqwest::Client>,
) -> Template {
    let theme = theme.unwrap_or("reader");
    let brand = BRANDS
        .iter()
        .map(|(id, _)| *id)
        .find(|id| Some(*id) == brand)
        .unwrap_or("wordpress");
    let font = FONTS
        .iter()
        .map(|(id, _)| *id)
        .find(|id| Some(*id) == font)
        .unwrap_or("sans");
    let size = size
        .filter(|size| size.is_finite() && *size > 0.0)
        .unwrap_or(1.0);
    let appearance = appearance
        .filter(|appearance| APPEARANCES.iter().any(|(id, _)| id == appearance))
        .unwrap_or("auto");
    let width = width.unwrap_or(390);
    let offline = offline.unwrap_or(false);
    let photon = photon.unwrap_or(true);
    let lang = lang.unwrap_or_default();

    let reader_post = match fetch_post(client, site, post).await {
        Ok(reader_post) => reader_post,
        Err(error) => {
            return Template::render("reader/index", context! { url: site, error });
        }
    };
    let content = &reader_post.content;
    let known_themes = themes();

    let frame = |theme: &str| {
        let query = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("theme", theme)
            .append_pair("brand", brand)
            .append_pair("offline", &offline.to_string())
            .append_pair("photon", &photon.to_string())
            .append_pair("lang", lang)
            .append_pair("font", font)
            .append_pair("size", &size.to_string())
            .finish();
        let name = known_themes
            .iter()
            .find(|(id, _)| id == theme)
            .map_or(theme, |(_, name)| *name);
        json!({ "name": name, "src": format!("/reader/{site}/{post}/page?{query}") })
    };
    let frames: Vec<Value> = if theme == "all" {
        known_themes.iter().map(|(id, _)| frame(id)).collect()
    } else {
        vec![frame(theme)]
    };
    let link = |changes: &[(&str, String)]| {
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        let current = [
            ("theme", theme.to_owned()),
            ("brand", brand.to_owned()),
            ("appearance", appearance.to_owned()),
            ("width", width.to_string()),
            ("offline", offline.to_string()),
            ("photon", photon.to_string()),
            ("lang", lang.to_owned()),
            ("font", font.to_owned()),
            ("size", size.to_string()),
        ];
        for (key, value) in current {
            let value = changes
                .iter()
                .find(|(changed, _)| *changed == key)
                .map_or(value, |(_, changed)| changed.clone());
            query.append_pair(key, &value);
        }
        format!("/reader/{site}/{post}?{}", query.finish())
    };
    let themes: Vec<Value> = known_themes
        .iter()
        .map(|(id, name)| (id.as_str(), *name))
        .chain([("all", "All themes")])
        .map(|(id, name)| {
            json!({ "name": name, "href": link(&[("theme", id.to_string())]), "selected": id == theme })
        })
        .collect();
    let fonts: Vec<Value> = FONTS
        .iter()
        .map(|(id, name)| {
            json!({ "name": name, "href": link(&[("font", id.to_string())]), "selected": *id == font })
        })
        .collect();
    let sizes: Vec<Value> = SIZES
        .iter()
        .map(|scale| {
            json!({ "name": format!("{scale}×"), "href": link(&[("size", scale.to_string())]), "selected": *scale == size })
        })
        .collect();
    let brands: Vec<Value> = BRANDS
        .iter()
        .map(|(id, name)| {
            json!({ "name": name, "href": link(&[("brand", id.to_string())]), "selected": *id == brand })
        })
        .collect();
    let appearances: Vec<Value> = APPEARANCES
        .iter()
        .map(|(id, name)| {
            json!({ "name": name, "href": link(&[("appearance", id.to_string())]), "selected": *id == appearance })
        })
        .collect();

    let images: Vec<Value> = content
        .images()
        .iter()
        .map(|image| {
            let size = match (image.width, image.height) {
                (Some(width), Some(height)) => format!("{width}×{height}"),
                _ => String::new(),
            };
            json!({ "url": image.url, "size": size, "sizes": image.sizes.len(), "caption": image.caption })
        })
        .collect();
    let anchors: Vec<Value> = content
        .anchors()
        .iter()
        .map(|anchor| {
            let kind = match anchor.kind {
                ReaderPostAnchorKind::Heading { level } => format!("Heading {level}"),
                ReaderPostAnchorKind::Footnote => "Footnote".to_owned(),
                ReaderPostAnchorKind::Other => "Linked to".to_owned(),
            };
            json!({ "id": anchor.id, "kind": kind, "text": anchor.text })
        })
        .collect();
    let galleries: Vec<usize> = content
        .galleries()
        .iter()
        .map(|gallery| gallery.images.len())
        .collect();
    let mentions: Vec<String> = content
        .mentions()
        .iter()
        .map(|mention| format!("@{}", mention.username))
        .collect();
    let excerpt = (!reader_post.excerpt.is_empty()).then(|| {
        if content.is_manual_excerpt(&reader_post.excerpt) {
            "The author's"
        } else {
            "Made from the post"
        }
    });
    let featured_image_in_post = (!reader_post.featured_image.is_empty())
        .then(|| content.includes_image(&reader_post.featured_image));

    Template::render(
        "reader/post",
        context! {
            site,
            post,
            title: text(&reader_post.title),
            url: reader_post.url,
            reader_url: reader_post.reader_url,
            frames,
            theme,
            all_themes: theme == "all",
            themes,
            brand,
            brands,
            appearance,
            appearances,
            font,
            fonts,
            size,
            sizes,
            // A frame's `color-scheme` is its page's `prefers-color-scheme`, so this forces the
            // appearance the way an app's web view does, without changing the page.
            color_scheme: (appearance != "auto").then_some(appearance),
            // A theme that always looks the same ignores a forced appearance, so the switch only
            // shows for one that follows the system, and for all of them side by side.
            always: match page_colors(theme, brand).appearance() {
                _ if theme == "all" => None,
                ReaderThemeAppearance::FollowsSystem => None,
                ReaderThemeAppearance::AlwaysLight => Some("Always light"),
                ReaderThemeAppearance::AlwaysDark => Some("Always dark"),
            },
            width,
            phone_href: link(&[("width", "390".to_owned())]),
            tablet_href: link(&[("width", "768".to_owned())]),
            offline,
            offline_href: link(&[("offline", (!offline).to_string())]),
            photon,
            photon_href: link(&[("photon", (!photon).to_string())]),
            site_is_private: reader_post.site_is_private,
            lang,
            scripts: content.scripts(),
            images,
            galleries,
            anchors,
            mentions,
            excerpt,
            featured_image_in_post,
            plain_text: content.plain_text(),
            cleaned_html: content.html(),
            raw_html: reader_post.raw_html,
        },
    )
}

/// The page the apps' web views would show, for the viewer's frames.
///
/// The page brings its own Content Security Policy, which only lets the scripts the apps would run
/// run: the shared script, and the embeds' scripts the cleanup kept. So even a post the cleanup
/// missed something in can't run its own scripts on this site. It keeps this site's origin, rather
/// than a sandboxed one: embeds like X's and Instagram's don't work without storage. The header
/// adds what a policy in the page can't: only this site can show it in a frame.
#[derive(Responder)]
#[response(content_type = "html")]
struct PreviewPage {
    html: String,
    policy: Header<'static>,
}

impl PreviewPage {
    fn new(html: String) -> Self {
        Self {
            html,
            policy: Header::new("Content-Security-Policy", "frame-ancestors 'self'"),
        }
    }

    /// An error instead of the page, which runs no scripts at all.
    fn error(message: &str) -> Self {
        Self {
            html: format!("<p>{}</p>", escape(message)),
            policy: Header::new(
                "Content-Security-Policy",
                "script-src 'none'; frame-ancestors 'self'",
            ),
        }
    }
}

/// What an app adds around the page natively: it draws the background behind the web view, and
/// lays the web view out with margins.
const APP_CSS: &str =
    "html { background-color: var(--reader-background); } body { padding: 16px; }";

/// The WordPress.com Reader's post text, from Calypso's `.reader-full-post__story-content`.
const READER_CSS: &str = r#"
html {
  font-size: calc(17px * var(--reader-text-scale, 1));
}
body {
  max-width: 720px;
  margin: 0 auto;
  /* Calypso's 28px, as a ratio, so larger text, like a large paragraph, gets room too. */
  line-height: 1.647;
}
@media (max-width: 480px) {
  html {
    font-size: calc(16px * var(--reader-text-scale, 1));
  }
  body {
    line-height: 1.5;
  }
}
p {
  margin-bottom: 16px;
}
"#;

/// The CSS for a theme, besides its colours.
fn theme_css(theme: &str) -> String {
    let reader = if theme == "reader" { READER_CSS } else { "" };
    format!("{APP_CSS}\n{reader}")
}

#[get("/reader/<site>/<post>/page?<theme>&<brand>&<font>&<size>&<offline>&<photon>&<lang>")]
#[allow(clippy::too_many_arguments)]
async fn page(
    site: &str,
    post: u64,
    theme: Option<&str>,
    brand: Option<&str>,
    font: Option<&str>,
    size: Option<f64>,
    offline: Option<bool>,
    photon: Option<bool>,
    lang: Option<&str>,
    client: &State<reqwest::Client>,
) -> PreviewPage {
    let reader_post = match fetch_post(client, site, post).await {
        Ok(reader_post) => reader_post,
        Err(error) => return PreviewPage::error(&error),
    };
    // Photon can't fetch a private site's images.
    let content = if photon.unwrap_or(true) && !reader_post.site_is_private {
        reader_post.content.with_photon_sizes()
    } else {
        reader_post.content
    };
    let html = content.page_html(ReaderPageOptions {
        colors: page_colors(theme.unwrap_or("reader"), brand.unwrap_or("wordpress")),
        language: lang.filter(|lang| !lang.is_empty()).map(str::to_owned),
        offline: offline.unwrap_or(false),
        font: Some(match font {
            Some("serif") => ReaderFont::Serif,
            Some("mono") => ReaderFont::Mono,
            _ => ReaderFont::Sans,
        }),
        text_scale: size.unwrap_or(1.0),
        server_css: None,
        css: Some(theme_css(theme.unwrap_or("reader"))),
    });
    PreviewPage::new(html)
}

struct ReaderPost {
    title: String,
    url: String,
    /// The post in WordPress.com's Reader, to compare with.
    reader_url: Option<String>,
    excerpt: String,
    featured_image: String,
    site_is_private: bool,
    raw_html: String,
    content: Arc<ReaderPostContent>,
}

async fn fetch_post(client: &reqwest::Client, site: &str, post: u64) -> Result<ReaderPost, String> {
    let json = get_json(
        client,
        &format!("{API}/v1.2/read/sites/{site}/posts/{post}?meta=site"),
    )
    .await?;
    let field = |name: &str| json[name].as_str().unwrap_or_default().to_owned();
    let raw_html = field("content");
    let url = field("URL");
    let content = ReaderPostContent::new(&raw_html, (!url.is_empty()).then(|| url.clone()));
    Ok(ReaderPost {
        title: field("title"),
        url,
        reader_url: calypso_post_url(&json),
        excerpt: field("excerpt"),
        featured_image: field("featured_image"),
        site_is_private: json["site_is_private"].as_bool().unwrap_or(false),
        raw_html,
        content,
    })
}

/// The post's page in WordPress.com's Reader, the way Calypso's `getPostUrl` makes it: by its feed
/// item if it has one, and otherwise by its site.
fn calypso_post_url(json: &Value) -> Option<String> {
    let id = |name: &str| json[name].as_u64().filter(|id| *id > 0);
    let path = match (id("feed_ID"), id("feed_item_ID")) {
        (Some(feed), Some(item)) => format!("feeds/{feed}/posts/{item}"),
        (Some(feed), None) if json["is_external"].as_bool() == Some(true) => {
            format!("feeds/{feed}/posts/{}", id("ID")?)
        }
        _ => format!("blogs/{}/posts/{}", id("site_ID")?, id("ID")?),
    };
    Some(format!("https://wordpress.com/reader/{path}"))
}

/// Where a site or post URL, a domain, or a site ID leads: a post, or the site's recent posts.
async fn resolve(client: &reqwest::Client, input: &str) -> Result<String, String> {
    if input.chars().all(|c| c.is_ascii_digit()) {
        return Ok(format!("/reader/{input}"));
    }
    let with_scheme = if input.contains("://") {
        input.to_owned()
    } else {
        format!("https://{input}")
    };
    let url = Url::parse(&with_scheme).map_err(|_| format!("{input} isn't a URL"))?;
    let host = url
        .host_str()
        .ok_or_else(|| format!("{input} has no host"))?;
    let site = match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    };

    // A post's address: `?p=123`, or a path ending in its slug.
    if let Some((_, id)) = url.query_pairs().find(|(key, _)| key == "p")
        && id.chars().all(|c| c.is_ascii_digit())
    {
        return Ok(format!("/reader/{site}/{id}"));
    }
    if let Some(slug) = url
        .path_segments()
        .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
    {
        let lookup = format!("{API}/v1.1/sites/{site}/posts/slug:{slug}?fields=ID");
        if let Some(id) = get_json(client, &lookup)
            .await
            .ok()
            .and_then(|json| json["ID"].as_u64())
        {
            return Ok(format!("/reader/{site}/{id}"));
        }
    }
    Ok(format!("/reader/{site}"))
}

async fn get_json(client: &reqwest::Client, url: &str) -> Result<Value, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| describe(&error))?;
    let status = response.status();
    let body = response.text().await.map_err(|error| describe(&error))?;
    let json: Value = serde_json::from_str(&body)
        .map_err(|_| format!("The Reader API sent something other than JSON ({status})"))?;
    if !status.is_success() {
        let message = json["message"].as_str().unwrap_or("no message");
        return Err(format!("The Reader API answered {status}: {message}"));
    }
    Ok(json)
}

fn page_colors(theme: &str, brand: &str) -> ReaderPageColors {
    if let Some(theme) = reader_fixed_theme_from_id(theme) {
        return ReaderPageColors::Fixed { theme };
    }
    // The themes that follow the system take their link colour from the app's brand.
    let branded = |theme_colors: fn(bool) -> ReaderThemeColors| {
        let colors = |dark| match brand {
            "jetpack" => ReaderThemeColors {
                link: jetpack_link(dark),
                ..theme_colors(dark)
            },
            _ => theme_colors(dark),
        };
        ReaderPageColors::System {
            light: colors(false),
            dark: colors(true),
        }
    };
    match theme {
        "system" => branded(system_colors),
        // The Reader theme, which is the default. Like the WordPress.com Reader, it follows the
        // system's appearance.
        _ => branded(reader_colors),
    }
}

/// Jetpack's link colour: Jetpack Green 50 in light mode, as in Calypso's Jetpack colour scheme, and
/// Green 30 in dark mode. The Green 40 the app uses elsewhere is too faint for text on white.
fn jetpack_link(dark: bool) -> ReaderColor {
    let (red, green, blue) = if dark {
        (0x2f, 0xb4, 0x1f)
    } else {
        (0x00, 0x87, 0x10)
    };
    ReaderColor {
        red,
        green,
        blue,
        alpha: 255,
    }
}

/// The System theme's colours. They come from each app, so these are close to iOS's, with the
/// WordPress app's blue for links.
fn system_colors(dark: bool) -> ReaderThemeColors {
    let color = |red, green, blue, alpha| ReaderColor {
        red,
        green,
        blue,
        alpha,
    };
    if dark {
        ReaderThemeColors {
            text: color(255, 255, 255, 255),
            secondary_text: color(235, 235, 245, 153),
            tertiary_text: color(235, 235, 245, 77),
            background: color(0, 0, 0, 255),
            secondary_background: color(28, 28, 30, 255),
            border: color(84, 84, 88, 153),
            link: color(57, 156, 227, 255),
        }
    } else {
        ReaderThemeColors {
            text: color(0, 0, 0, 255),
            secondary_text: color(60, 60, 67, 153),
            tertiary_text: color(60, 60, 67, 77),
            background: color(255, 255, 255, 255),
            secondary_background: color(242, 242, 247, 255),
            border: color(60, 60, 67, 74),
            link: color(6, 117, 196, 255),
        }
    }
}

/// The WordPress.com Reader's colours, from Calypso: its default colour scheme, with Color Studio's
/// grays and WordPress blue, and its dark mode.
fn reader_colors(dark: bool) -> ReaderThemeColors {
    let color = |hex: u32| ReaderColor {
        red: (hex >> 16) as u8,
        green: (hex >> 8) as u8,
        blue: hex as u8,
        alpha: 255,
    };
    if dark {
        // Calypso's dark mode, which maps these to the Dashboard's dark colours.
        return ReaderThemeColors {
            // `--dashboard__text-color`.
            text: color(0xe0e0e0),
            // `--dashboard__text-muted-color`.
            secondary_text: color(0xbdbdbd),
            // Calypso has no third text colour, so this is between the muted text and the page.
            tertiary_text: color(0x8a8a8a),
            // `--dashboard__background-color`.
            background: color(0x1e1e1e),
            // `--dashboard-surface__background-color`.
            secondary_background: color(0x2a2a2a),
            // `--dashboard-surface__border-color`.
            border: color(0x4a4a4a),
            // `--wp-admin-theme-color-darker-20`.
            link: color(0x9bbcfb),
        };
    }
    ReaderThemeColors {
        // `--color-neutral-70`, which the post's text uses.
        text: color(0x3c434a),
        // `--color-text-subtle`.
        secondary_text: color(0x646970),
        tertiary_text: color(0x8c8f94),
        background: color(0xffffff),
        // `--color-neutral-0`.
        secondary_background: color(0xf6f7f7),
        // `--color-border-subtle`.
        border: color(0xdcdcde),
        // `--color-link`.
        link: color(0x3858e9),
    }
}

/// Text from the Reader API's HTML, like a title with `&#8217;` in it.
fn text(html: &str) -> String {
    ReaderPostContent::new(html, None).plain_text()
}

/// An error, with what caused it: `reqwest`'s own message leaves out why a request failed.
fn describe(error: &dyn std::error::Error) -> String {
    let mut description = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        description += &format!(": {cause}");
        source = cause.source();
    }
    description
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
