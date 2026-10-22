//! URLs, by what they point at: the post's own site or another WordPress site, an image or a file on
//! any site, Photon's copy of an image, an embed provider or CDN, or a page on any other site.
//!
//! On WordPress sites, a path keeps its shape: WordPress makes it, and the library reads it, like a
//! WordPress.com post's `/2024/05/31/a-post/` or a mention's `/mentions/someone/`. On an embed
//! provider's or CDN's host, a path keeps its words, like `embed`. Anywhere else, all the library
//! reads is whether it's `/` and what kind of file it ends in, so that's all a structure keeps.

use std::sync::LazyLock;

use ::url::Url as WebUrl;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::attribute::Decimal;
use super::vocabulary::{Extension, KnownHost, PathWord, QueryKey, SchemeName, ScriptFile};
use super::{
    Anchor, FileId, ImageId, KeyId, Names, NumberId, PageId, SegmentId, SiteId, SlugId, TokenId,
    UserId, ValueId, Vocabulary,
};
use crate::reader::images::image_key;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Url {
    Empty,
    /// `#a1`: a link to an element in the same page.
    Anchor(Anchor),
    /// `#`, which goes nowhere.
    Hash,
    Mailto,
    Tel,
    Sms,
    Javascript,
    /// A `data:` URL, which only has to match itself.
    Data(DataUrl, TokenId),
    /// A URL with a scheme other than HTTP's, like `ftp:`.
    OtherScheme(SchemeName),
    /// A URL that doesn't parse, like `///a`.
    Unparsed,
    /// A page on the post's site or another WordPress site.
    Page {
        scheme: Scheme,
        site: Site,
        path: SitePath,
        #[serde(default)]
        query: Vec<QueryParam>,
        #[serde(default)]
        fragment: Option<Fragment>,
    },
    /// An image on any site but an embed provider's or CDN's. Each size of an image is the same
    /// image.
    Image {
        scheme: Scheme,
        site: Site,
        image: ImageId,
        file: FileName,
        #[serde(default)]
        query: Vec<QueryParam>,
    },
    /// Any other file, like a PDF or a video, on any site but an embed provider's or CDN's.
    File {
        scheme: Scheme,
        site: Site,
        file: FileId,
        name: FileName,
        #[serde(default)]
        query: Vec<QueryParam>,
    },
    /// Photon's copy of an image: `https://i0.wp.com/<the image's host and path>`.
    Photon {
        scheme: Scheme,
        host: PhotonHost,
        origin: Box<Url>,
        #[serde(default)]
        query: Vec<QueryParam>,
    },
    /// An embed provider's or CDN's URL, like a YouTube video's.
    Provider {
        scheme: Scheme,
        www: bool,
        host: KnownHost,
        path: Vec<Segment>,
        #[serde(default)]
        query: Vec<QueryParam>,
        #[serde(default)]
        fragment: Option<Fragment>,
    },
    /// A page on a site that isn't a WordPress site, an embed provider's or a CDN's.
    Anywhere {
        scheme: Scheme,
        page: AnywherePage,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Scheme {
    Https,
    Http,
    /// `//example.com/`.
    ProtocolRelative,
    /// `/a/path`, on the post's site.
    Relative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Site {
    /// The post's own site.
    This { www: bool, hosting: Hosting },
    /// WordPress.com's own pages, like the Reader's.
    WordPressDotCom { www: bool },
    /// Another WordPress.com site.
    WordPressCom(SiteId),
    /// A WordPress.com site's uploads, on `files.wordpress.com`.
    Files(SiteId),
    /// A WordPress.com staging site.
    Staging(SiteId),
    /// Any other site. Only images and files keep this: a page there is a [`Url::Anywhere`].
    Anywhere,
}

/// What follows a URL's `#`: an element's ID, or nothing, which goes nowhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Fragment {
    Empty,
    Anchor(Anchor),
}

/// Where the post's own site is, since the library treats a WordPress.com site's links to its
/// posts differently.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Hosting {
    Own,
    WordPressCom,
    Staging,
}

impl Hosting {
    pub(crate) fn of(host: &str) -> Hosting {
        if host.ends_with(".wordpress.com") {
            Hosting::WordPressCom
        } else if host.ends_with(".wpcomstaging.com") {
            Hosting::Staging
        } else {
            Hosting::Own
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PhotonHost {
    I0,
    I1,
    I2,
    I3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DataUrl {
    Gif,
    Png,
    Svg,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AnywherePage {
    /// Just the site's address, like `https://example.com/`.
    Root(PageId),
    Page(PageId),
}

/// A file's name, without the name: its extension, and the size and edit WordPress adds to the
/// names of an image's copies, like `-1024x768`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct FileName {
    #[serde(default)]
    pub(crate) size: Option<(u32, u32)>,
    #[serde(default)]
    pub(crate) variant: Option<Variant>,
    #[serde(default)]
    pub(crate) edited: bool,
    pub(crate) extension: Extension,
}

/// The suffix WordPress gives a large or turned original.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Variant {
    Scaled,
    Rotated,
}

/// A path on a WordPress site.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct SitePath {
    pub(crate) route: Route,
    #[serde(default)]
    pub(crate) trailing_slash: bool,
}

/// What a path on a WordPress site is, from WordPress's permalinks and routes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Route {
    Root,
    /// The post's own path, in the shape of its permalink.
    ThisPost(Permalink),
    /// `/2024/05/31/a-post/`.
    DayAndName(SlugId),
    /// `/2024/05/a-post/`.
    MonthAndName(SlugId),
    Tag(SlugId),
    Category(SlugId),
    Author(SlugId),
    /// `/mentions/someone/`.
    Mentions(UserId),
    /// `/a-page/` or `/a-page/its-child/`: only slugs.
    Page(Vec<SlugId>),
    Other(Vec<SiteSegment>),
}

/// The shape of the post's own address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Permalink {
    /// `/2024/05/31/a-post/`.
    DayAndName,
    /// `/2024/05/a-post/`.
    MonthAndName,
    Other,
}

impl Permalink {
    pub(crate) fn of(path: &str) -> Permalink {
        let segments: Vec<&str> = path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();
        let digits = |segment: &str, length: usize| {
            segment.len() == length && segment.chars().all(|c| c.is_ascii_digit())
        };
        match segments.as_slice() {
            [year, month, day, _] if digits(year, 4) && digits(month, 2) && digits(day, 2) => {
                Permalink::DayAndName
            }
            [year, month, _] if digits(year, 4) && digits(month, 2) => Permalink::MonthAndName,
            _ => Permalink::Other,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SiteSegment {
    Word(PathWord),
    Year,
    Month,
    Day,
    Number(NumberId),
    Slug(SlugId),
    User(UserId),
}

/// A segment of an embed provider's or CDN's path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Segment {
    Empty,
    Word(PathWord),
    Script(ScriptFile),
    StandIn(SegmentId, Shape),
    File(SegmentId, Shape, FileName),
}

/// What a stand-in keeps of what it stands for: whether it's a number or hexadecimal, like a
/// gist's ID, since the library reads that.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Shape {
    Digits,
    Hex,
    Other,
}

impl Shape {
    fn of(text: &str) -> Shape {
        if !text.is_empty() && text.chars().all(|c| c.is_ascii_digit()) {
            Shape::Digits
        } else if !text.is_empty() && text.chars().all(|c| c.is_ascii_hexdigit()) {
            Shape::Hex
        } else {
            Shape::Other
        }
    }

    fn render(self, id: SegmentId) -> String {
        match self {
            Shape::Digits => (1000 + id.0).to_string(),
            Shape::Hex => format!("f{}", id.0),
            Shape::Other => format!("x{}", id.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct QueryParam {
    pub(crate) key: Key,
    pub(crate) value: QueryValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Key {
    Known(QueryKey),
    Other(KeyId),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum QueryValue {
    Empty,
    Number(Decimal),
    /// A size, like Photon's `resize=1024,768`.
    Numbers(Vec<Decimal>),
    Other(ValueId),
}

// --- Encoding -------------------------------------------------------------------------------------

/// Where a URL's host puts it.
enum Host {
    Site(Site),
    Photon(PhotonHost),
    Provider { www: bool, host: KnownHost },
}

impl Url {
    /// The URL `url` stands in as.
    pub(crate) fn encode(names: &mut Names, url: &str) -> Url {
        let url = url.trim();
        if url.is_empty() {
            return Url::Empty;
        }
        if let Some(fragment) = url.strip_prefix('#') {
            return if fragment.is_empty() {
                Url::Hash
            } else {
                Url::Anchor(Anchor::of(names, fragment))
            };
        }
        let lower = url.to_ascii_lowercase();
        for (scheme, url) in [
            ("mailto:", Url::Mailto),
            ("tel:", Url::Tel),
            ("sms:", Url::Sms),
            ("javascript:", Url::Javascript),
        ] {
            if lower.starts_with(scheme) {
                return url;
            }
        }
        if let Some(rest) = lower.strip_prefix("data:") {
            let kind = match rest.split([';', ',']).next().unwrap_or_default() {
                "image/gif" => DataUrl::Gif,
                "image/png" => DataUrl::Png,
                "image/svg+xml" => DataUrl::Svg,
                _ => DataUrl::Other,
            };
            return Url::Data(kind, TokenId::of(names, url));
        }
        let base = WebUrl::parse("https://relative.invalid/").expect("a valid URL");
        // A URL with an extra slash, like `///example.com/a.jpg`, doesn't parse, but it does once
        // the cleanup gives it a scheme, as it does any URL that starts with `//`.
        let parsed = WebUrl::options()
            .base_url(Some(&base))
            .parse(url)
            .or_else(|error| match url.strip_prefix("//") {
                Some(_) => WebUrl::parse(&format!("https:{url}")),
                None => Err(error),
            });
        let Ok(parsed) = parsed else {
            return Url::Unparsed;
        };
        if !matches!(parsed.scheme(), "http" | "https") {
            return Url::OtherScheme(
                SchemeName::parse_or_other(parsed.scheme()).expect("it has a stand-in"),
            );
        }
        let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
        let scheme = if host == "relative.invalid" {
            Scheme::Relative
        } else if url.starts_with("//") {
            Scheme::ProtocolRelative
        } else if parsed.scheme() == "http" {
            Scheme::Http
        } else {
            Scheme::Https
        };
        let site = match where_host(names, &host, scheme) {
            Host::Photon(photon) => return photon_url(names, scheme, photon, &parsed),
            Host::Provider { www, host } => {
                return Url::Provider {
                    scheme,
                    www,
                    host,
                    path: provider_path(names, parsed.path()),
                    query: query(names, &parsed),
                    fragment: fragment(names, &parsed),
                };
            }
            Host::Site(site) => site,
        };

        let path = parsed.path();
        let last = path.rsplit('/').next().unwrap_or_default();
        if let Some((_, file)) = FileName::parse(last) {
            // The image's key: its host and path without its size, so its copies match.
            let key_host = match site {
                Site::This { .. } => "post.example",
                _ => &host,
            };
            let key = format!("https://{key_host}{path}");
            if file.extension.is_image() {
                let key = image_key(&key).unwrap_or(key);
                return Url::Image {
                    scheme,
                    site,
                    image: ImageId::of(names, &key),
                    file,
                    query: query(names, &parsed),
                };
            }
            return Url::File {
                scheme,
                site,
                file: FileId::of(names, &key),
                name: file,
                query: query(names, &parsed),
            };
        }
        if site == Site::Anywhere {
            let id = PageId::of(names, parsed.as_str());
            return Url::Anywhere {
                scheme,
                page: if path == "/" {
                    AnywherePage::Root(id)
                } else {
                    AnywherePage::Page(id)
                },
            };
        }
        let is_this = matches!(site, Site::This { .. });
        Url::Page {
            scheme,
            site,
            path: SitePath::encode(names, path, is_this),
            query: query(names, &parsed),
            fragment: fragment(names, &parsed),
        }
    }
}

fn where_host(names: &mut Names, host: &str, scheme: Scheme) -> Host {
    let bare = host.trim_start_matches("www.");
    let www = bare.len() < host.len();
    if scheme == Scheme::Relative || Some(bare) == names.post_host.as_deref() {
        let hosting = names.post_host.as_deref().map_or(Hosting::Own, Hosting::of);
        return Host::Site(Site::This { www, hosting });
    }
    if let Some(photon) = photon_host(host) {
        return Host::Photon(photon);
    }
    if bare == "wordpress.com" {
        return Host::Site(Site::WordPressDotCom { www });
    }
    if host.ends_with(".files.wordpress.com") && host != "videos.files.wordpress.com" {
        return Host::Site(Site::Files(SiteId::of(names, host)));
    }
    if let Some(known) = KnownHost::parse(bare) {
        return Host::Provider { www, host: known };
    }
    if host.ends_with(".wordpress.com") {
        return Host::Site(Site::WordPressCom(SiteId::of(names, host)));
    }
    if host.ends_with(".wpcomstaging.com") {
        return Host::Site(Site::Staging(SiteId::of(names, host)));
    }
    Host::Site(Site::Anywhere)
}

fn photon_host(host: &str) -> Option<PhotonHost> {
    match host {
        "i0.wp.com" => Some(PhotonHost::I0),
        "i1.wp.com" => Some(PhotonHost::I1),
        "i2.wp.com" => Some(PhotonHost::I2),
        "i3.wp.com" => Some(PhotonHost::I3),
        _ => None,
    }
}

/// Photon's path starts with the image's host.
fn photon_url(names: &mut Names, scheme: Scheme, host: PhotonHost, parsed: &WebUrl) -> Url {
    let origin = parsed.path().trim_start_matches('/');
    Url::Photon {
        scheme,
        host,
        origin: Box::new(if origin.is_empty() {
            Url::Empty
        } else {
            Url::encode(names, &format!("https://{origin}"))
        }),
        query: query(names, parsed),
    }
}

fn fragment(names: &mut Names, parsed: &WebUrl) -> Option<Fragment> {
    parsed.fragment().map(|fragment| {
        if fragment.is_empty() {
            Fragment::Empty
        } else {
            Fragment::Anchor(Anchor::of(names, fragment))
        }
    })
}

fn query(names: &mut Names, parsed: &WebUrl) -> Vec<QueryParam> {
    parsed
        .query_pairs()
        .map(|(key, value)| QueryParam {
            key: match QueryKey::parse(&key) {
                Some(known) => Key::Known(known),
                None => Key::Other(KeyId::of(names, &key)),
            },
            value: if value.is_empty() {
                QueryValue::Empty
            } else if let Some(number) = is_small_number(&value)
                .then(|| Decimal::new(&value))
                .flatten()
            {
                QueryValue::Number(number)
            } else if SIZE_PAIR.is_match(&value) {
                QueryValue::Numbers(value.split(',').filter_map(Decimal::new).collect())
            } else {
                QueryValue::Other(ValueId::of(names, &value))
            },
        })
        .collect()
}

pub(super) fn is_small_number(value: &str) -> bool {
    !value.is_empty() && value.len() <= 5 && value.chars().all(|c| c.is_ascii_digit() || c == '.')
}

static SIZE_PAIR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d{1,5}(,\d{1,5}){0,3}$").expect("a valid regex"));

impl Extension {
    fn is_image(self) -> bool {
        matches!(
            self.as_str(),
            "jpg"
                | "jpeg"
                | "jpe"
                | "png"
                | "gif"
                | "webp"
                | "avif"
                | "heic"
                | "heif"
                | "svg"
                | "bmp"
                | "tif"
                | "tiff"
                | "ico"
        )
    }
}

impl FileName {
    /// A file's name without its extension, and what it says, if it ends in an extension.
    pub(crate) fn parse(file: &str) -> Option<(&str, FileName)> {
        let (mut stem, extension) = file.rsplit_once('.')?;
        let extension = Extension::parse(&extension.to_ascii_lowercase())?;
        let mut name = FileName {
            size: None,
            variant: None,
            edited: false,
            extension,
        };
        loop {
            if let Some(captures) = SIZE_SUFFIX.captures(stem) {
                let width = captures[1].parse().ok()?;
                let height = captures[2].parse().ok()?;
                name.size.get_or_insert((width, height));
                stem = &stem[..captures.get(0).expect("it matched").start()];
            } else if let Some(rest) = stem.strip_suffix("-scaled") {
                name.variant.get_or_insert(Variant::Scaled);
                stem = rest;
            } else if let Some(rest) = stem.strip_suffix("-rotated") {
                name.variant.get_or_insert(Variant::Rotated);
                stem = rest;
            } else if let Some(found) = EDIT_SUFFIX.find(stem) {
                name.edited = true;
                stem = &stem[..found.start()];
            } else {
                return Some((stem, name));
            }
        }
    }

    pub(crate) fn render(&self, stem: &str) -> String {
        let mut file = stem.to_owned();
        if self.edited {
            file += "-e100000";
        }
        if let Some((width, height)) = self.size {
            file += &format!("-{width}x{height}");
        }
        match self.variant {
            Some(Variant::Scaled) => file += "-scaled",
            Some(Variant::Rotated) => file += "-rotated",
            None => {}
        }
        format!("{file}.{}", self.extension.as_str())
    }
}

static SIZE_SUFFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-(\d{1,5})x(\d{1,5})$").expect("a valid regex"));
/// An edited image's suffix, like `-e1678320800368`, which says when it was edited.
static EDIT_SUFFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-e\d{6,}$").expect("a valid regex"));

impl SitePath {
    fn encode(names: &mut Names, path: &str, is_this: bool) -> SitePath {
        let trailing_slash = path.len() > 1 && path.ends_with('/');
        if is_this
            && names
                .post_path
                .as_deref()
                .is_some_and(|post| post.trim_end_matches('/') == path.trim_end_matches('/'))
        {
            return SitePath {
                route: Route::ThisPost(Permalink::of(path)),
                trailing_slash,
            };
        }
        let raw: Vec<&str> = path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();
        let mut segments = Vec::new();
        for segment in &raw {
            let previous = segments.last().copied();
            let is_number = segment.chars().all(|c| c.is_ascii_digit());
            let number = || segment.parse::<u32>().unwrap_or(0);
            segments.push(match previous {
                Some(SiteSegment::Word(PathWord::Mentions)) => {
                    SiteSegment::User(UserId::of(names, &segment.to_ascii_lowercase()))
                }
                _ if is_number && segment.len() == 4 && (1995..=2035).contains(&number()) => {
                    SiteSegment::Year
                }
                Some(SiteSegment::Year)
                    if is_number && segment.len() == 2 && (1..=12).contains(&number()) =>
                {
                    SiteSegment::Month
                }
                Some(SiteSegment::Month)
                    if is_number && segment.len() == 2 && (1..=31).contains(&number()) =>
                {
                    SiteSegment::Day
                }
                _ if is_number => SiteSegment::Number(NumberId::of(names, segment)),
                _ => match PathWord::parse(segment) {
                    Some(word) => SiteSegment::Word(word),
                    None => SiteSegment::Slug(SlugId::of(names, segment)),
                },
            });
        }
        use SiteSegment::{Day, Month, Slug, User, Word, Year};
        let route = match segments.as_slice() {
            [] => Route::Root,
            [Year, Month, Day, Slug(slug)] => Route::DayAndName(*slug),
            [Year, Month, Slug(slug)] => Route::MonthAndName(*slug),
            [Word(PathWord::Tag), Slug(slug)] => Route::Tag(*slug),
            [Word(PathWord::Category), Slug(slug)] => Route::Category(*slug),
            [Word(PathWord::Author), Slug(slug)] => Route::Author(*slug),
            [Word(PathWord::Mentions), User(user)] => Route::Mentions(*user),
            slugs if slugs.iter().all(|segment| matches!(segment, Slug(_))) => Route::Page(
                slugs
                    .iter()
                    .filter_map(|segment| match segment {
                        Slug(slug) => Some(*slug),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => Route::Other(segments),
        };
        SitePath {
            route,
            trailing_slash,
        }
    }

    fn render(&self) -> String {
        let slug = |slug: &SlugId| format!("p{}", slug.0);
        let segments: Vec<String> = match &self.route {
            Route::Root => return "/".to_owned(),
            Route::ThisPost(Permalink::DayAndName) => {
                vec!["2020".into(), "01".into(), "01".into(), "post".into()]
            }
            Route::ThisPost(Permalink::MonthAndName) => {
                vec!["2020".into(), "01".into(), "post".into()]
            }
            Route::ThisPost(Permalink::Other) => vec!["post".to_owned()],
            Route::DayAndName(name) => vec!["2020".into(), "01".into(), "01".into(), slug(name)],
            Route::MonthAndName(name) => vec!["2020".into(), "01".into(), slug(name)],
            Route::Tag(name) => vec!["tag".into(), slug(name)],
            Route::Category(name) => vec!["category".into(), slug(name)],
            Route::Author(name) => vec!["author".into(), slug(name)],
            Route::Mentions(user) => vec!["mentions".into(), format!("user{}", user.0)],
            Route::Page(slugs) => slugs.iter().map(slug).collect(),
            Route::Other(segments) => segments
                .iter()
                .map(|segment| match segment {
                    SiteSegment::Word(word) => word.as_str().to_owned(),
                    SiteSegment::Year => "2020".to_owned(),
                    SiteSegment::Month | SiteSegment::Day => "01".to_owned(),
                    SiteSegment::Number(number) => (1000 + number.0).to_string(),
                    SiteSegment::Slug(name) => slug(name),
                    SiteSegment::User(user) => format!("user{}", user.0),
                })
                .collect(),
        };
        let slash = if self.trailing_slash { "/" } else { "" };
        format!("/{}{slash}", segments.join("/"))
    }
}

/// An embed provider's or CDN's path: its words, like `embed`, stay, and anything else, like a
/// video's ID, becomes a stand-in. A file keeps its extension.
fn provider_path(names: &mut Names, path: &str) -> Vec<Segment> {
    let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let count = segments.len();
    segments
        .into_iter()
        .enumerate()
        .map(|(index, segment)| {
            if segment.is_empty() {
                Segment::Empty
            } else if let Some(word) = PathWord::parse(segment) {
                Segment::Word(word)
            } else if let Some(script) = ScriptFile::parse(segment) {
                Segment::Script(script)
            } else {
                match FileName::parse(segment).filter(|_| index + 1 == count) {
                    Some((stem, file)) => {
                        Segment::File(SegmentId::of(names, stem), Shape::of(stem), file)
                    }
                    None => Segment::StandIn(SegmentId::of(names, segment), Shape::of(segment)),
                }
            }
        })
        .collect()
}

// --- Rendering ------------------------------------------------------------------------------------

impl Url {
    /// The URL the structure's stand-ins make, or a real one in the preview's
    /// [`with_real_media`](super::media::with_real_media).
    pub(crate) fn render(&self) -> String {
        if super::media::is_on()
            && let Some(url) = super::media::real_url(self)
        {
            return url;
        }
        match self {
            Url::Empty => String::new(),
            Url::Anchor(anchor) => format!("#{}", anchor.render()),
            Url::Hash => "#".to_owned(),
            Url::Mailto => "mailto:someone@example.com".to_owned(),
            Url::Tel => "tel:+10000000000".to_owned(),
            Url::Sms => "sms:+10000000000".to_owned(),
            Url::Javascript => "javascript:void(0)".to_owned(),
            // A fragment, which a `data:` URL's content doesn't include, tells them apart.
            Url::Data(kind, id) => format!(
                "{}#d{}",
                match kind {
                    DataUrl::Gif => {
                        "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"
                    }
                    DataUrl::Png => {
                        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII="
                    }
                    DataUrl::Svg =>
                        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg'/%3E",
                    DataUrl::Other => "data:,",
                },
                id.0
            ),
            Url::OtherScheme(scheme) => format!("{}:x", scheme.as_str()),
            Url::Unparsed => "///x".to_owned(),
            Url::Page {
                scheme,
                site,
                path,
                query,
                fragment,
            } => format!(
                "{}{}{}{}",
                origin(*scheme, &site_host(*site)),
                path.render(),
                render_query(query),
                render_fragment(*fragment)
            ),
            Url::Image {
                scheme,
                site,
                image,
                file,
                query,
            } => format!(
                "{}{}{}{}",
                origin(*scheme, &site_host(*site)),
                uploads(*site),
                file.render(&format!("i{}", image.0)),
                render_query(query)
            ),
            Url::File {
                scheme,
                site,
                file,
                name,
                query,
            } => format!(
                "{}{}{}{}",
                origin(*scheme, &site_host(*site)),
                uploads(*site),
                name.render(&format!("f{}", file.0)),
                render_query(query)
            ),
            Url::Photon {
                scheme,
                host,
                origin: image,
                query,
            } => {
                let image = image.render();
                let image = image
                    .strip_prefix("https://")
                    .or_else(|| image.strip_prefix("http://"))
                    .or_else(|| image.strip_prefix("//"))
                    .unwrap_or(&image);
                let host = match host {
                    PhotonHost::I0 => "i0.wp.com",
                    PhotonHost::I1 => "i1.wp.com",
                    PhotonHost::I2 => "i2.wp.com",
                    PhotonHost::I3 => "i3.wp.com",
                };
                format!("{}/{image}{}", origin(*scheme, host), render_query(query))
            }
            Url::Provider {
                scheme,
                www,
                host,
                path,
                query,
                fragment,
            } => {
                let host = format!("{}{}", if *www { "www." } else { "" }, host.as_str());
                let path: Vec<String> = path
                    .iter()
                    .map(|segment| match segment {
                        Segment::Empty => String::new(),
                        Segment::Word(word) => word.as_str().to_owned(),
                        Segment::Script(script) => script.as_str().to_owned(),
                        Segment::StandIn(id, shape) => shape.render(*id),
                        Segment::File(id, shape, file) => file.render(&shape.render(*id)),
                    })
                    .collect();
                format!(
                    "{}/{}{}{}",
                    origin(*scheme, &host),
                    path.join("/"),
                    render_query(query),
                    render_fragment(*fragment)
                )
            }
            Url::Anywhere { scheme, page } => match page {
                AnywherePage::Root(id) => format!(
                    "{}/",
                    origin(*scheme, &format!("u{}.anywhere.invalid", id.0))
                ),
                AnywherePage::Page(id) => {
                    format!("{}/u{}", origin(*scheme, "anywhere.invalid"), id.0)
                }
            },
        }
    }
}

fn origin(scheme: Scheme, host: &str) -> String {
    match scheme {
        Scheme::Https => format!("https://{host}"),
        Scheme::Http => format!("http://{host}"),
        Scheme::ProtocolRelative => format!("//{host}"),
        Scheme::Relative => String::new(),
    }
}

fn site_host(site: Site) -> String {
    match site {
        Site::This { www, hosting } => format!(
            "{}{}",
            if www { "www." } else { "" },
            match hosting {
                Hosting::Own => "post.example",
                Hosting::WordPressCom => "this_post.wordpress.com",
                Hosting::Staging => "this_post.wpcomstaging.com",
            }
        ),
        Site::WordPressDotCom { www } => format!("{}wordpress.com", if www { "www." } else { "" }),
        // Underscores, which a WordPress.com site's name can't have, so they're no one's.
        Site::WordPressCom(id) => format!("site_{}.wordpress.com", id.0),
        Site::Files(id) => format!("site_{}.files.wordpress.com", id.0),
        Site::Staging(id) => format!("site_{}.wpcomstaging.com", id.0),
        Site::Anywhere => "anywhere.invalid".to_owned(),
    }
}

/// Where a site keeps its uploads.
fn uploads(site: Site) -> &'static str {
    match site {
        Site::Files(_) => "/2020/01/",
        Site::Anywhere => "/",
        _ => "/wp-content/uploads/2020/01/",
    }
}

fn render_query(query: &[QueryParam]) -> String {
    if query.is_empty() {
        return String::new();
    }
    let pairs: Vec<String> = query
        .iter()
        .map(|param| {
            let key = match param.key {
                Key::Known(key) => key.as_str().to_owned(),
                Key::Other(id) => format!("k{}", id.0),
            };
            let value = match &param.value {
                QueryValue::Empty => String::new(),
                QueryValue::Number(number) => number.as_str().to_owned(),
                QueryValue::Numbers(numbers) => numbers
                    .iter()
                    .map(Decimal::as_str)
                    .collect::<Vec<_>>()
                    .join(","),
                QueryValue::Other(id) => format!("v{}", id.0),
            };
            format!("{key}={value}")
        })
        .collect();
    format!("?{}", pairs.join("&"))
}

fn render_fragment(fragment: Option<Fragment>) -> String {
    match fragment {
        None => String::new(),
        Some(Fragment::Empty) => "#".to_owned(),
        Some(Fragment::Anchor(anchor)) => format!("#{}", anchor.render()),
    }
}
