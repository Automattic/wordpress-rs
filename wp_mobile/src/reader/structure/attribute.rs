//! Attributes, each with the type of its value: a URL, text, a number or size, a keyword from a
//! vocabulary, or a stand-in. An attribute that isn't in a table goes.
//!
//! The tables nest, by where an attribute comes from: [`Attribute`] has HTML's own, [`Aria`] the
//! accessibility ones, and [`Data`] the `data-` ones, with Jetpack's Carousel's, lazy loading's,
//! embeds' and the Interactivity API's in tables of their own.

use std::borrow::Cow;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::class::Class;
use super::text::Text;
use super::url::{Url, is_small_number};
use super::vocabulary::{
    Align, AllowFeature, AnimatedAttribute, AriaCurrent, Charset, Conversation, CrossOrigin,
    CssUnit, Decoding, Dir, DirectiveKind, DirectiveSuffix, EmbedTheme, FetchPriority, FlagValue,
    FrameBorder, HttpEquiv, ItemProp, Lang, LinkType, Loading, MediaType, ObjectFit, Paint, PinDo,
    Preload, ReferrerPolicy, Rel, Role, SandboxToken, SchemaType, Scope, Scrolling, SizeProperty,
    Tag, Target, Valign, Xmlns,
};
use super::{Anchor, AttachmentId, Names, SecretId, TokenId, UserId, Vocabulary};

/// An attribute's value, made from the attribute's text.
pub(crate) trait Value: Sized {
    /// The value `value` stands in as, or `None` if the attribute goes.
    fn encode(names: &mut Names, tag: Tag, attribute: &str, value: &str) -> Option<Self>;
    fn render(&self, attribute: &str) -> String;

    /// The URLs in the value, if it has any.
    fn urls(&self) -> Vec<&Url> {
        Vec::new()
    }
}

/// A table of attributes.
pub(crate) trait Table: Sized {
    /// `None` if the table doesn't have the attribute, and `Some(None)` if it does, but the
    /// attribute goes.
    fn encode(names: &mut Names, tag: Tag, name: &str, value: &str) -> Option<Option<Self>>;
    fn render(&self) -> (Cow<'static, str>, String);
    /// The attributes the table has.
    fn names() -> Vec<&'static str>;
    /// The URLs in the attribute's value, if it has any.
    fn urls(&self) -> Vec<&Url>;
}

macro_rules! attributes {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $( $(#[$variant_meta:meta])* $variant:ident($attribute:literal, $value:ty), )*
            $( ..$nested:ident($table:ty), )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::enum_variant_names)]
        $vis enum $name {
            $( $(#[$variant_meta])* #[serde(rename = $attribute)] $variant($value), )*
            $( $nested($table), )*
        }

        impl Table for $name {
            fn encode(names: &mut Names, tag: Tag, name: &str, value: &str) -> Option<Option<Self>> {
                match name {
                    $( $attribute => return Some(
                        <$value as Value>::encode(names, tag, $attribute, value).map(Self::$variant)
                    ), )*
                    _ => {}
                }
                $(
                    if let Some(found) = <$table as Table>::encode(names, tag, name, value) {
                        return Some(found.map(Self::$nested));
                    }
                )*
                None
            }

            fn render(&self) -> (Cow<'static, str>, String) {
                match self {
                    $( Self::$variant(value) => (Cow::Borrowed($attribute), <$value as Value>::render(value, $attribute)), )*
                    $( Self::$nested(table) => table.render(), )*
                }
            }

            fn names() -> Vec<&'static str> {
                #[allow(unused_mut)]
                let mut names = vec![$($attribute),*];
                $( names.extend(<$table as Table>::names()); )*
                names
            }

            fn urls(&self) -> Vec<&Url> {
                match self {
                    $( Self::$variant(value) => <$value as Value>::urls(value), )*
                    $( Self::$nested(table) => table.urls(), )*
                }
            }
        }
    };
}

attributes! {
    pub(crate) enum Attribute {
        Href("href", Url),
        Src("src", Url),
        Poster("poster", Url),
        Cite("cite", Url),
        Action("action", Url),
        Background("background", Url),
        Longdesc("longdesc", Url),
        Srcset("srcset", Srcset),
        Class("class", Classes),
        Style("style", Style),
        Alt("alt", Text),
        Title("title", Text),
        Placeholder("placeholder", Text),
        Id("id", AnchorIds),
        For("for", AnchorIds),
        Headers("headers", AnchorIds),
        Name("name", AnchorIds),
        Width("width", Length),
        Height("height", Length),
        Colspan("colspan", Decimal),
        Rowspan("rowspan", Decimal),
        Span("span", Decimal),
        Start("start", Decimal),
        Border("border", Decimal),
        Cellpadding("cellpadding", Decimal),
        Cellspacing("cellspacing", Decimal),
        Marginwidth("marginwidth", Decimal),
        Marginheight("marginheight", Decimal),
        Tabindex("tabindex", Decimal),
        Version("version", Decimal),
        Cx("cx", Length),
        Cy("cy", Length),
        R("r", Length),
        X("x", Length),
        Y("y", Length),
        ViewBox("viewbox", ViewBox),
        Reversed("reversed", Flag),
        Controls("controls", Flag),
        Autoplay("autoplay", Flag),
        Muted("muted", Flag),
        Loop("loop", Flag),
        Playsinline("playsinline", Flag),
        Allowfullscreen("allowfullscreen", Flag),
        Itemscope("itemscope", Flag),
        Hidden("hidden", Flag),
        Open("open", Flag),
        Nowrap("nowrap", Flag),
        Focusable("focusable", Flag),
        Dir("dir", Keyword<Dir>),
        Lang("lang", Keyword<Lang>),
        Type("type", Keyword<MediaType>),
        Loading("loading", Keyword<Loading>),
        Decoding("decoding", Keyword<Decoding>),
        FetchPriority("fetchpriority", Keyword<FetchPriority>),
        Preload("preload", Keyword<Preload>),
        Frameborder("frameborder", Keyword<FrameBorder>),
        CrossOrigin("crossorigin", Keyword<CrossOrigin>),
        ReferrerPolicy("referrerpolicy", Keyword<ReferrerPolicy>),
        Scrolling("scrolling", Keyword<Scrolling>),
        Scope("scope", Keyword<Scope>),
        Align("align", Keyword<Align>),
        Valign("valign", Keyword<Valign>),
        Charset("charset", Keyword<Charset>),
        HttpEquiv("http-equiv", Keyword<HttpEquiv>),
        Xmlns("xmlns", Keyword<Xmlns>),
        Fill("fill", Keyword<Paint>),
        Stroke("stroke", Keyword<Paint>),
        Target("target", Keyword<Target>),
        Role("role", Keyword<Role>),
        Allow("allow", Features),
        Sandbox("sandbox", Words<SandboxToken>),
        Rel("rel", Words<Rel>),
        Itemprop("itemprop", Words<ItemProp>),
        Itemtype("itemtype", ItemType),
        Sizes("sizes", Sizes),
        Media("media", Media),
        Datetime("datetime", Datetime),
        D("d", PathData),
        Content("content", Content),
        /// What an SVG animation changes, which the cleanup reads to stop one changing a link.
        AttributeName("attributename", Keyword<AnimatedAttribute>),
        ..Aria(Aria),
        ..Data(Data),
    }
}

attributes! {
    /// Accessibility's attributes.
    pub(crate) enum Aria {
        Label("aria-label", Text),
        Hidden("aria-hidden", Flag),
        Expanded("aria-expanded", Flag),
        Modal("aria-modal", Flag),
        Current("aria-current", Keyword<AriaCurrent>),
        Level("aria-level", Decimal),
        Labelledby("aria-labelledby", AnchorIds),
        Describedby("aria-describedby", AnchorIds),
        Controls("aria-controls", AnchorIds),
    }
}

attributes! {
    /// `data-` attributes.
    pub(crate) enum Data {
        Url("data-url", Url),
        Href("data-href", Url),
        Link("data-link", Url),
        FullUrl("data-full-url", Url),
        /// A link's target, which Gutenberg keeps here, or an attachment's ID.
        Id("data-id", DataId),
        Type("data-type", Keyword<LinkType>),
        Username("data-username", Username),
        Fn("data-fn", AnchorIds),
        Width("data-width", Length),
        Height("data-height", Length),
        OriginalWidth("data-original-width", Decimal),
        OriginalHeight("data-original-height", Decimal),
        Ratio("data-ratio", Decimal),
        RecalcDims("data-recalc-dims", Decimal),
        Columns("data-columns", Decimal),
        ObjectFit("data-object-fit", Keyword<ObjectFit>),
        IsImage("data-is-image", Flag),
        ..Carousel(Carousel),
        ..Lazy(Lazy),
        ..Embed(Embed),
        ..Interactivity(Directive),
    }
}

attributes! {
    /// What Jetpack's Carousel keeps on each image.
    pub(crate) enum Carousel {
        OrigFile("data-orig-file", Url),
        LargeFile("data-large-file", Url),
        MediumFile("data-medium-file", Url),
        Permalink("data-permalink", Url),
        OrigSize("data-orig-size", SizePair),
        ImageTitle("data-image-title", Text),
        ImageCaption("data-image-caption", Text),
        ImageDescription("data-image-description", Text),
        AttachmentId("data-attachment-id", Attachment),
        CommentsOpened("data-comments-opened", Decimal),
    }
}

attributes! {
    /// Where lazy-loading plugins keep an image's address until it loads.
    pub(crate) enum Lazy {
        Src("data-src", Url),
        LazySrc("data-lazy-src", Url),
        Original("data-original", Url),
        OrigSrc("data-orig-src", Url),
        Srcset("data-srcset", Srcset),
        LazySrcset("data-lazy-srcset", Srcset),
    }
}

attributes! {
    /// What embeds' markup and scripts read.
    pub(crate) enum Embed {
        WpcomEmbedUrl("data-wpcom-embed-url", Url),
        InstagramPermalink("data-instgrm-permalink", Url),
        InstagramVersion("data-instgrm-version", Decimal),
        InstagramCaptioned("data-instgrm-captioned", Flag),
        PinDo("data-pin-do", Keyword<PinDo>),
        Dnt("data-dnt", Flag),
        Conversation("data-conversation", Keyword<Conversation>),
        Theme("data-theme", Keyword<EmbedTheme>),
        Lang("data-lang", Keyword<Lang>),
        EmbedHeight("data-embed-height", Length),
        /// What pairs a WordPress embed's iframe with its blockquote, which `reader.js` matches.
        Secret("data-secret", Secret),
        BlueskyUri("data-bluesky-uri", BlueskyUri),
        BlueskyCid("data-bluesky-cid", BlueskyCid),
        /// A TikTok video's ID, which its embed script loads the video from.
        TikTokVideoId("data-video-id", TikTokVideoId),
        /// A gist's JSON, which `reader.js` loads into the code block the cleanup makes. A post's
        /// own is a safety case: the cleanup removes it.
        ReaderGist("data-reader-gist", Url),
    }
}

// --- The Interactivity API ------------------------------------------------------------------------

/// One of the Interactivity API's directives, like `data-wp-on--click`. The cleanup only cares that
/// they're there, and their values name code, keys and data, so they all become `x`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Directive {
    kind: DirectiveKind,
    #[serde(default)]
    suffix: Option<DirectiveSuffix>,
}

impl Table for Directive {
    fn encode(_: &mut Names, _: Tag, name: &str, _: &str) -> Option<Option<Self>> {
        let rest = name.strip_prefix("data-wp-")?;
        let (kind, suffix) = match rest.split_once("--") {
            Some((kind, suffix)) => (kind, Some(suffix)),
            None => (rest, None),
        };
        Some((|| {
            let kind = DirectiveKind::parse(kind)?;
            let suffix = match suffix {
                Some(suffix) => Some(DirectiveSuffix::parse_or_other(suffix)?),
                None => None,
            };
            Some(Directive { kind, suffix })
        })())
    }

    fn render(&self) -> (Cow<'static, str>, String) {
        let suffix = self
            .suffix
            .map(|suffix| format!("--{}", suffix.as_str()))
            .unwrap_or_default();
        (
            Cow::Owned(format!("data-wp-{}{suffix}", self.kind.as_str())),
            "x".to_owned(),
        )
    }

    fn names() -> Vec<&'static str> {
        Vec::new()
    }

    fn urls(&self) -> Vec<&Url> {
        Vec::new()
    }
}

// --- Values ---------------------------------------------------------------------------------------

impl Value for Url {
    fn encode(names: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        Some(Url::encode(names, value))
    }

    fn render(&self, _: &str) -> String {
        Url::render(self)
    }

    fn urls(&self) -> Vec<&Url> {
        vec![self]
    }
}

impl Value for Text {
    fn encode(names: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        Some(Text::encode(names, value))
    }

    fn render(&self, _: &str) -> String {
        Text::render(self)
    }
}

/// A number: digits, perhaps a sign and decimals. A number of six digits or more, like an ID,
/// becomes `0`, and decimals stop at four.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct Decimal(String);

impl Decimal {
    pub(crate) fn new(text: &str) -> Option<Decimal> {
        let text = text.trim();
        if !DECIMAL.is_match(text) {
            return None;
        }
        let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
        let digits = whole.trim_start_matches('-');
        let whole = if digits.len() >= 6 { "0" } else { whole };
        let fraction = &fraction[..fraction.len().min(4)];
        Some(Decimal(if fraction.is_empty() {
            whole.to_owned()
        } else {
            format!("{whole}.{fraction}")
        }))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Decimal {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Decimal::new(&text)
            .filter(|decimal| decimal.0 == text)
            .ok_or_else(|| format!("{text:?} isn't a number"))
    }
}

impl From<Decimal> for String {
    fn from(decimal: Decimal) -> Self {
        decimal.0
    }
}

impl Value for Decimal {
    fn encode(_: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        Decimal::new(value)
    }

    fn render(&self, _: &str) -> String {
        self.0.clone()
    }
}

static DECIMAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^-?(\d{1,12}(\.\d{1,12})?|\.\d{1,12})$").expect("a valid regex"));

/// A CSS length, like `640`, `100%` or `1.5em`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Length {
    number: Decimal,
    #[serde(default)]
    unit: Option<CssUnit>,
}

impl Length {
    pub(crate) fn parse(text: &str) -> Option<Length> {
        let captures = LENGTH.captures(text.trim())?;
        Some(Length {
            number: Decimal::new(&captures[1])?,
            unit: match captures.get(2) {
                Some(unit) => Some(CssUnit::parse(unit.as_str())?),
                None => None,
            },
        })
    }

    pub(crate) fn render(&self) -> String {
        format!(
            "{}{}",
            self.number.0,
            self.unit.map(CssUnit::as_str).unwrap_or_default()
        )
    }
}

impl Value for Length {
    fn encode(_: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        Length::parse(value)
    }

    fn render(&self, _: &str) -> String {
        Length::render(self)
    }
}

static LENGTH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(-?(?:\d+(?:\.\d+)?|\.\d+))\s*([a-z%]+)?$").expect("a valid regex")
});

/// A size, like `data-orig-size`'s `4032,3024`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct SizePair(Decimal, Decimal);

impl Value for SizePair {
    fn encode(_: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        let (width, height) = value.split_once(',')?;
        Some(SizePair(Decimal::new(width)?, Decimal::new(height)?))
    }

    fn render(&self, _: &str) -> String {
        format!("{},{}", self.0.0, self.1.0)
    }
}

/// An SVG's `viewBox`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ViewBox([Decimal; 4]);

impl Value for ViewBox {
    fn encode(_: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        let numbers: Vec<Decimal> = value
            .split([' ', ','])
            .filter(|number| !number.is_empty())
            .map(Decimal::new)
            .collect::<Option<_>>()?;
        Some(ViewBox(numbers.try_into().ok()?))
    }

    fn render(&self, _: &str) -> String {
        self.0
            .iter()
            .map(Decimal::as_str)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// A yes-or-no attribute's value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Flag {
    Value(FlagValue),
    /// The attribute's own name, like `controls="controls"`.
    Named,
}

impl Value for Flag {
    fn encode(_: &mut Names, _: Tag, attribute: &str, value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case(attribute) {
            return Some(Flag::Named);
        }
        FlagValue::parse_or_other(value.trim()).map(Flag::Value)
    }

    fn render(&self, attribute: &str) -> String {
        match self {
            Flag::Value(value) => value.as_str().to_owned(),
            Flag::Named => attribute.to_owned(),
        }
    }
}

/// A keyword from a vocabulary. One the vocabulary doesn't have becomes its `x`, if it has one, and
/// otherwise the attribute goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Keyword<T>(T);

fn keyword<T: Vocabulary>(word: &str) -> Option<T> {
    T::parse(word)
        .or_else(|| T::parse(&word.to_ascii_lowercase()))
        .or_else(T::other)
}

impl<T: Vocabulary> Value for Keyword<T> {
    fn encode(_: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        keyword(value.trim()).map(Keyword)
    }

    fn render(&self, _: &str) -> String {
        self.0.as_str().to_owned()
    }
}

/// Keywords from a vocabulary, between spaces, like a link's `rel`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Words<T>(Vec<T>);

impl<T: Vocabulary> Value for Words<T> {
    fn encode(_: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        let words: Vec<&str> = value.split_whitespace().collect();
        let kept: Vec<T> = words.iter().filter_map(|word| keyword(word)).collect();
        // An empty `sandbox` allows nothing, which isn't the same as none.
        (words.is_empty() || !kept.is_empty()).then_some(Words(kept))
    }

    fn render(&self, _: &str) -> String {
        self.0
            .iter()
            .map(|word| word.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// What an iframe's `allow` lets it use, between semicolons.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Features(Vec<AllowFeature>);

impl Value for Features {
    fn encode(_: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        Some(Features(
            value
                .split(';')
                .filter_map(|feature| feature.split_whitespace().next())
                .filter_map(keyword)
                .collect(),
        ))
    }

    fn render(&self, _: &str) -> String {
        self.0
            .iter()
            .map(|feature| feature.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// IDs between spaces: an element's own, or the ones a label or a link points at.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct AnchorIds(Vec<Anchor>);

impl AnchorIds {
    pub(crate) fn anchors(&self) -> &[Anchor] {
        &self.0
    }
}

impl Value for AnchorIds {
    fn encode(names: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        Some(AnchorIds(
            value
                .split_whitespace()
                .map(|id| Anchor::of(names, id))
                .collect(),
        ))
    }

    fn render(&self, _: &str) -> String {
        self.0
            .iter()
            .map(|id| id.render())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// An element's classes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Classes(Vec<Class>);

impl Classes {
    pub(crate) fn classes(&self) -> &[Class] {
        &self.0
    }
}

impl Value for Classes {
    fn encode(names: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        let classes: Vec<Class> = value
            .split_whitespace()
            .filter_map(|class| Class::encode(names, class))
            .collect();
        Some(Classes(if classes.is_empty() && !value.trim().is_empty() {
            vec![Class::Other]
        } else {
            classes
        }))
    }

    fn render(&self, _: &str) -> String {
        self.0
            .iter()
            .map(|class| class.render())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// An image's sources for each size, in `srcset`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Srcset(Vec<Candidate>);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Candidate {
    url: Url,
    #[serde(default)]
    descriptor: Option<Descriptor>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Descriptor {
    /// `1024w`.
    Width(Decimal),
    /// `2x`.
    Density(Decimal),
}

impl Value for Srcset {
    fn encode(names: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        Some(Srcset(
            value
                .split(',')
                .map(|candidate| {
                    let mut parts = candidate.split_whitespace();
                    let url = Url::encode(names, parts.next().unwrap_or_default());
                    let descriptor = parts.find_map(|part| {
                        if let Some(width) = part.strip_suffix('w') {
                            Decimal::new(width).map(Descriptor::Width)
                        } else if let Some(density) = part.strip_suffix('x') {
                            Decimal::new(density).map(Descriptor::Density)
                        } else {
                            None
                        }
                    });
                    Candidate { url, descriptor }
                })
                .collect(),
        ))
    }

    fn render(&self, _: &str) -> String {
        self.0
            .iter()
            .map(|candidate| {
                let url = candidate.url.render();
                match &candidate.descriptor {
                    Some(Descriptor::Width(width)) => format!("{url} {}w", width.0),
                    Some(Descriptor::Density(density)) => format!("{url} {}x", density.0),
                    None => url,
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn urls(&self) -> Vec<&Url> {
        self.0.iter().map(|candidate| &candidate.url).collect()
    }
}

/// What `sizes` says: how wide an image shows at each screen width.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Sizes(Vec<SizesEntry>);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct SizesEntry {
    #[serde(default)]
    condition: Option<Media>,
    /// `None` for `auto`.
    #[serde(default)]
    size: Option<Length>,
}

impl Value for Sizes {
    fn encode(_: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        let entries: Vec<SizesEntry> = value
            .split(',')
            .filter_map(|entry| {
                let entry = entry.trim();
                let (condition, size) = match entry.rsplit_once(')') {
                    Some((condition, size)) => {
                        (Some(Media::parse(&format!("{condition})"))?), size)
                    }
                    None => (None, entry),
                };
                let size = size.trim();
                let size = if size == "auto" {
                    None
                } else {
                    Some(Length::parse(size)?)
                };
                Some(SizesEntry { condition, size })
            })
            .collect();
        (!entries.is_empty()).then_some(Sizes(entries))
    }

    fn render(&self, _: &str) -> String {
        self.0
            .iter()
            .map(|entry| {
                let size = entry
                    .size
                    .as_ref()
                    .map_or_else(|| "auto".to_owned(), Length::render);
                match &entry.condition {
                    Some(condition) => format!("{} {size}", condition.render("")),
                    None => size,
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// A media query for a width, like `(max-width: 1024px)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Media {
    max: bool,
    width: Length,
}

impl Media {
    fn parse(text: &str) -> Option<Media> {
        let captures = MEDIA.captures(text.trim())?;
        Some(Media {
            max: &captures[1] == "max",
            width: Length::parse(&captures[2])?,
        })
    }
}

impl Value for Media {
    fn encode(_: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        Media::parse(value)
    }

    fn render(&self, _: &str) -> String {
        format!(
            "({}-width: {})",
            if self.max { "max" } else { "min" },
            self.width.render()
        )
    }
}

static MEDIA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\(\s*(max|min)-width\s*:\s*([\d.]+\s*(?:px|em|rem|vw)?)\s*\)$")
        .expect("a valid regex")
});

/// Only the properties the cleanup reads for sizes, before it removes every style.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Style(Vec<Declaration>);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Declaration {
    property: SizeProperty,
    value: StyleValue,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StyleValue {
    /// One length, or a shorthand's few, like `padding: 56.25% 0 0`.
    Lengths(Vec<Length>),
    /// `aspect-ratio: 16 / 9`.
    Ratio(Decimal, Decimal),
}

impl Value for Style {
    fn encode(_: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        let declarations: Vec<Declaration> = value
            .split(';')
            .filter_map(|declaration| {
                let (property, value) = declaration.split_once(':')?;
                let property = SizeProperty::parse(&property.trim().to_ascii_lowercase())?;
                let value = value.trim();
                let value = match value.split_once('/') {
                    Some((width, height)) => {
                        StyleValue::Ratio(Decimal::new(width)?, Decimal::new(height)?)
                    }
                    None => {
                        let lengths: Vec<Length> = value
                            .split_whitespace()
                            .map(Length::parse)
                            .collect::<Option<_>>()?;
                        if lengths.is_empty() || lengths.len() > 4 {
                            return None;
                        }
                        StyleValue::Lengths(lengths)
                    }
                };
                Some(Declaration { property, value })
            })
            .collect();
        (!declarations.is_empty()).then_some(Style(declarations))
    }

    fn render(&self, _: &str) -> String {
        self.0
            .iter()
            .map(|declaration| {
                let value = match &declaration.value {
                    StyleValue::Lengths(lengths) => lengths
                        .iter()
                        .map(Length::render)
                        .collect::<Vec<_>>()
                        .join(" "),
                    StyleValue::Ratio(width, height) => format!("{} / {}", width.0, height.0),
                };
                format!("{}: {value}", declaration.property.as_str())
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// A schema.org type, in `itemtype`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ItemType {
    secure: bool,
    kind: SchemaType,
}

impl Value for ItemType {
    fn encode(_: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        let (scheme, kind) = value.trim().split_once("://schema.org/")?;
        Some(ItemType {
            secure: match scheme {
                "https" => true,
                "http" => false,
                _ => return None,
            },
            kind: SchemaType::parse(kind)?,
        })
    }

    fn render(&self, _: &str) -> String {
        format!(
            "{}://schema.org/{}",
            if self.secure { "https" } else { "http" },
            self.kind.as_str()
        )
    }
}

/// Microdata's value, like a rating's: a number, or something else, which becomes `x`. Only on the
/// elements microdata puts it on.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Content(Option<Decimal>);

impl Value for Content {
    fn encode(_: &mut Names, tag: Tag, _: &str, value: &str) -> Option<Self> {
        matches!(tag, Tag::Meta | Tag::Span).then(|| {
            Content(
                is_small_number(value)
                    .then(|| Decimal::new(value))
                    .flatten(),
            )
        })
    }

    fn render(&self, _: &str) -> String {
        self.0
            .as_ref()
            .map_or_else(|| "x".to_owned(), |number| number.0.clone())
    }
}

/// A date, which could say when a post was written, so it's always the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Datetime;

impl Value for Datetime {
    fn encode(_: &mut Names, _: Tag, _: &str, _: &str) -> Option<Self> {
        Some(Datetime)
    }

    fn render(&self, _: &str) -> String {
        "2020-01-01T00:00:00+00:00".to_owned()
    }
}

/// An icon's shape, which could be anything's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct PathData;

impl Value for PathData {
    fn encode(_: &mut Names, _: Tag, _: &str, _: &str) -> Option<Self> {
        Some(PathData)
    }

    fn render(&self, _: &str) -> String {
        "M0 0h24v24H0z".to_owned()
    }
}

/// Gutenberg's `data-id`: a link's target, or an attachment's ID.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DataId {
    Url(Url),
    Attachment(AttachmentId),
    Other,
}

impl Value for DataId {
    fn encode(names: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        let value = value.trim();
        Some(if value.starts_with("http") || value.starts_with("//") {
            DataId::Url(Url::encode(names, value))
        } else if !value.is_empty() && value.chars().all(|c| c.is_ascii_digit()) {
            DataId::Attachment(AttachmentId::of(names, value))
        } else {
            DataId::Other
        })
    }

    fn render(&self, _: &str) -> String {
        match self {
            DataId::Url(url) => url.render(),
            DataId::Attachment(id) => id.0.to_string(),
            DataId::Other => "x".to_owned(),
        }
    }

    fn urls(&self) -> Vec<&Url> {
        match self {
            DataId::Url(url) => vec![url],
            _ => Vec::new(),
        }
    }
}

/// An attachment's ID, like `data-attachment-id`'s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Attachment(AttachmentId);

impl Value for Attachment {
    fn encode(names: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        let value = value.trim();
        (!value.is_empty() && value.chars().all(|c| c.is_ascii_digit()))
            .then(|| Attachment(AttachmentId::of(names, value)))
    }

    fn render(&self, _: &str) -> String {
        self.0.0.to_string()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Username(UserId);

impl Value for Username {
    fn encode(names: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        Some(Username(UserId::of(names, &value.to_ascii_lowercase())))
    }

    fn render(&self, _: &str) -> String {
        format!("user{}", self.0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Secret(SecretId);

impl Value for Secret {
    fn encode(names: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        Some(Secret(SecretId::of(names, value)))
    }

    fn render(&self, _: &str) -> String {
        format!("s{}", self.0.0)
    }
}

/// A Bluesky post's `at://` URI, which its embed script loads the post from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct BlueskyUri(TokenId);

impl Value for BlueskyUri {
    fn encode(names: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        Some(BlueskyUri(TokenId::of(names, value)))
    }

    fn render(&self, _: &str) -> String {
        if super::media::is_on() {
            return super::media::bluesky_uri(self.0);
        }
        format!("at://did:plc:t{0}/app.bsky.feed.post/t{0}", self.0.0)
    }
}

/// A Bluesky post's content ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct BlueskyCid(TokenId);

impl Value for BlueskyCid {
    fn encode(names: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        Some(BlueskyCid(TokenId::of(names, value)))
    }

    fn render(&self, _: &str) -> String {
        format!("t{}", self.0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct TikTokVideoId(TokenId);

impl Value for TikTokVideoId {
    fn encode(names: &mut Names, _: Tag, _: &str, value: &str) -> Option<Self> {
        Some(TikTokVideoId(TokenId::of(names, value)))
    }

    fn render(&self, _: &str) -> String {
        if super::media::is_on() {
            return super::media::tiktok_id(self.0).to_owned();
        }
        (1000 + self.0.0).to_string()
    }
}
