//! A post's structure, as Rust types: its elements, the classes WordPress and its plugins give them,
//! and the shape of its text and URLs, but nothing that could identify the post, its site or its
//! author.
//!
//! Every name in a structure comes from a closed vocabulary, an enum in [`vocabulary`], and every
//! value is typed: a number, a size, a keyword from a vocabulary, or a numbered stand-in, like the
//! third site the post links to. The few strings left are checked when they're made and when
//! they're read back: punctuation, which can't hold a letter or a digit, and numbers. So a
//! structure can only hold what its types can, and the types are where to look to see what that is.
//!
//! [`Structure::of`] makes a post's structure, and [`Structure::post`] makes HTML from it again,
//! with stand-ins for what was dropped, so the library can run on it.

use std::collections::HashMap;

use scraper::{ElementRef, Html, Node as HtmlNode};
use serde::{Deserialize, Serialize};

use super::corpus_tests::Post;

/// A closed set of names, like HTML's tags or Gutenberg's blocks, as an enum whose variants each
/// stand for one name. Serialized as the name in a human-readable format like JSON, and as its
/// index in a binary one.
macro_rules! vocabulary {
    ($(#[$meta:meta])* $vis:vis enum $name:ident { $($variant:ident = $text:literal),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[allow(clippy::enum_variant_names)]
        $vis enum $name {
            $($variant),*
        }

        impl super::Vocabulary for $name {
            const ALL: &'static [Self] = &[$(Self::$variant),*];

            fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),*
                }
            }

            fn parse(text: &str) -> Option<Self> {
                match text {
                    $($text => Some(Self::$variant),)*
                    _ => None,
                }
            }

            fn index(self) -> usize {
                self as usize
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                super::serialize_word(*self, serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                super::deserialize_word(deserializer)
            }
        }
    };
}
/// A closed set of names. See [`vocabulary!`].
pub(crate) trait Vocabulary: Copy + 'static {
    const ALL: &'static [Self];
    fn as_str(self) -> &'static str;
    fn parse(text: &str) -> Option<Self>;
    fn index(self) -> usize;

    /// The word that stands for any value the vocabulary doesn't have, if it has one: `x`.
    fn other() -> Option<Self> {
        Self::parse("x")
    }

    /// The word for `text`, or the vocabulary's stand-in for a word it doesn't have.
    fn parse_or_other(text: &str) -> Option<Self> {
        Self::parse(text).or_else(Self::other)
    }
}

fn serialize_word<T: Vocabulary, S: serde::Serializer>(
    word: T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if serializer.is_human_readable() {
        serializer.serialize_str(word.as_str())
    } else {
        serializer.serialize_u16(word.index() as u16)
    }
}

fn deserialize_word<'de, T: Vocabulary, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<T, D::Error> {
    struct Visitor<T>(std::marker::PhantomData<T>);
    impl<T: Vocabulary> serde::de::Visitor<'_> for Visitor<T> {
        type Value = T;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a word from the vocabulary")
        }
        fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<T, E> {
            T::parse(text).ok_or_else(|| E::custom(format!("{text:?} isn't in the vocabulary")))
        }
        fn visit_u64<E: serde::de::Error>(self, index: u64) -> Result<T, E> {
            T::ALL
                .get(index as usize)
                .copied()
                .ok_or_else(|| E::custom(format!("{index} isn't in the vocabulary")))
        }
    }
    if deserializer.is_human_readable() {
        deserializer.deserialize_str(Visitor(std::marker::PhantomData))
    } else {
        deserializer.deserialize_u16(Visitor(std::marker::PhantomData))
    }
}

/// Numbered stand-ins, like the third site a post links to. Each kind counts from 1 in each post.
macro_rules! stand_ins {
    ($($(#[$meta:meta])* $name:ident => $kind:ident;)*) => {
        $(
            $(#[$meta])*
            #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
            #[serde(transparent)]
            pub(crate) struct $name(pub(crate) u32);

            impl $name {
                pub(crate) fn of(names: &mut Names, key: &str) -> Self {
                    Self(names.number(Kind::$kind, key))
                }
            }
        )*

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub(crate) enum Kind {
            $($kind),*
        }
    };
}

stand_ins! {
    /// An element's `id`, and the links and labels that point at it.
    AnchorId => Anchor;
    /// A site the post links to, other than its own.
    SiteId => Site;
    /// A slug in a path, like a post's or a tag's.
    SlugId => Slug;
    /// A number in a path, like a site's or a post's ID.
    NumberId => Number;
    /// An image, the same for each of its sizes and for Photon's copies of it.
    ImageId => Image;
    /// A file that isn't an image.
    FileId => File;
    /// A page on a site that isn't a WordPress site.
    PageId => Page;
    /// A username, from a mention or after an `@`.
    UserId => User;
    /// What pairs a WordPress embed's iframe with its blockquote.
    SecretId => Secret;
    /// A segment of an embed provider's or CDN's path, like a video's ID.
    SegmentId => Segment;
    /// A query key no vocabulary has.
    KeyId => Key;
    /// A query value that isn't a number.
    ValueId => Value;
    /// A media library attachment, like `wp-image-1234`'s.
    AttachmentId => Attachment;
    /// Anything else that only has to match itself, like a Bluesky post's URI.
    TokenId => Token;
}

/// An element's ID, or a link to one: a stand-in, with the prefix the library reads, if it has
/// one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Anchor {
    Plain(AnchorId),
    /// `comment-123`, which `reader_link` opens as the comment.
    Comment(AnchorId),
    /// `file-hello-rb`, a gist's file.
    File(AnchorId),
}

impl Anchor {
    pub(crate) fn of(names: &mut Names, id: &str) -> Anchor {
        let number = AnchorId::of(names, id);
        if id
            .strip_prefix("comment-")
            .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
        {
            Anchor::Comment(number)
        } else if id.starts_with("file-") {
            Anchor::File(number)
        } else {
            Anchor::Plain(number)
        }
    }

    pub(crate) fn render(self) -> String {
        match self {
            Anchor::Plain(id) => format!("a{}", id.0),
            Anchor::Comment(id) => format!("comment-{}", id.0),
            Anchor::File(id) => format!("file-a{}", id.0),
        }
    }
}

/// The stand-ins for one post's names, so each maps the same way everywhere in the post.
pub(crate) struct Names {
    /// The post's host, without `www.`.
    pub(crate) post_host: Option<String>,
    pub(crate) post_path: Option<String>,
    numbers: HashMap<(Kind, String), u32>,
    next: HashMap<Kind, u32>,
    /// What makes this post's stand-in words differ from other posts'.
    pub(crate) salt: String,
    /// Whether the text so far ends with an `@`, so the next word is a username.
    pub(crate) after_at: bool,
}

impl Names {
    fn for_post(url: &str) -> Self {
        let parsed = ::url::Url::parse(url).ok();
        Names {
            post_host: parsed.as_ref().and_then(|url| url.host_str()).map(|host| {
                host.to_ascii_lowercase()
                    .trim_start_matches("www.")
                    .to_owned()
            }),
            post_path: parsed.as_ref().map(|url| url.path().to_owned()),
            numbers: HashMap::new(),
            next: HashMap::new(),
            salt: url.to_owned(),
            after_at: false,
        }
    }

    /// The number `key` stands in as, among this post's names of the same kind.
    fn number(&mut self, kind: Kind, key: &str) -> u32 {
        if let Some(number) = self.numbers.get(&(kind, key.to_owned())) {
            return *number;
        }
        let next = self.next.entry(kind).or_insert(0);
        *next += 1;
        self.numbers.insert((kind, key.to_owned()), *next);
        *next
    }
}

// The modules come after the macros, which they use.
mod attribute;
mod class;
pub(super) mod media;
pub(super) mod patterns;
mod text;
mod url;
mod vocabulary;

pub(super) use attribute::Attribute;
use attribute::Table;
use text::{GettyLoad, Text};
pub(super) use url::Url;
use vocabulary::Tag;

// --- The tree -------------------------------------------------------------------------------------

/// A post's structure: its content and excerpt as trees, and its URL and featured image.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(super) struct Structure {
    pub(super) url: Url,
    pub(super) featured_image: Url,
    pub(super) content: Vec<Node>,
    pub(super) excerpt: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Node {
    Element(Element),
    Text(Text),
    Comment(Comment),
    /// Getty's inline script, which the cleanup builds the embed from.
    Getty(Vec<GettyLoad>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(super) struct Element {
    pub(super) tag: Tag,
    #[serde(default)]
    pub(super) attributes: Vec<Attribute>,
    #[serde(default)]
    pub(super) children: Vec<Node>,
}

/// A comment. The cleanup removes them, except the ones that split a post, but a paragraph that
/// holds only comments goes with them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Comment {
    More,
    NextPage,
    NoTeaser,
    Other,
}

impl Structure {
    /// The structure of `post`.
    pub(super) fn of(post: &Post) -> Self {
        let mut names = Names::for_post(&post.url);
        Structure {
            url: Url::encode(&mut names, &post.url),
            featured_image: Url::encode(&mut names, &post.featured_image),
            content: tree(&mut names, &post.content),
            excerpt: tree(&mut names, &post.excerpt),
        }
    }

    /// The post the structure stands for, with HTML made from its trees.
    pub(super) fn post(&self, id: &str) -> Post {
        Post {
            id: id.to_owned(),
            url: self.url.render(),
            content: render(&self.content),
            excerpt: render(&self.excerpt),
            featured_image: self.featured_image.render(),
            features: Vec::new(),
        }
    }
}

fn tree(names: &mut Names, html: &str) -> Vec<Node> {
    let document = Html::parse_fragment(html);
    children(names, document.root_element())
}

fn children(names: &mut Names, element: ElementRef<'_>) -> Vec<Node> {
    let mut children = Vec::new();
    let mut nodes: Vec<_> = element.children().collect();
    let mut index = 0;
    while index < nodes.len() {
        let child = nodes[index];
        index += 1;
        let node = match child.value() {
            // A `<template>`'s contents, which the parser keeps in a fragment of their own.
            HtmlNode::Fragment => {
                let contents: Vec<_> = child.children().collect();
                nodes.splice(index..index, contents);
                None
            }
            HtmlNode::Text(text) => Some(Node::Text(Text::encode(names, text))),
            HtmlNode::Element(_) => {
                ElementRef::wrap(child).map(|child| self::element(names, child))
            }
            HtmlNode::Comment(comment) => Some(Node::Comment(match comment.trim() {
                "more" => Comment::More,
                "nextpage" => Comment::NextPage,
                "noteaser" => Comment::NoTeaser,
                _ => Comment::Other,
            })),
            _ => None,
        };
        children.extend(node);
    }
    children
}

fn element(names: &mut Names, element: ElementRef<'_>) -> Node {
    let tag = Tag::parse(element.value().name()).unwrap_or(Tag::CustomElement);
    let attributes = element
        .value()
        .attrs()
        .filter_map(|(name, value)| {
            Attribute::encode(names, tag, &name.to_ascii_lowercase(), value).flatten()
        })
        .collect();
    let children = match tag {
        Tag::Script => {
            let code: String = element.text().collect();
            let loads = text::getty(names, &code);
            if loads.is_empty() {
                Vec::new()
            } else {
                vec![Node::Getty(loads)]
            }
        }
        // What a style says isn't structure.
        Tag::Style => Vec::new(),
        _ => children(names, element),
    };
    Node::Element(Element {
        tag,
        attributes,
        children,
    })
}

/// HTML from a structure's tree.
pub(super) fn render(nodes: &[Node]) -> String {
    let mut html = String::new();
    for node in nodes {
        render_node(node, &mut html);
    }
    html
}

fn render_node(node: &Node, html: &mut String) {
    match node {
        Node::Text(text) => *html += &escape(&text.render()),
        Node::Comment(comment) => {
            *html += match comment {
                Comment::More => "<!--more-->",
                Comment::NextPage => "<!--nextpage-->",
                Comment::NoTeaser => "<!--noteaser-->",
                Comment::Other => "<!-- -->",
            }
        }
        Node::Getty(loads) => *html += &text::render_getty(loads),
        Node::Element(element) => {
            let tag = element.tag.as_str();
            *html += "<";
            *html += tag;
            for attribute in &element.attributes {
                let (name, value) = attribute.render();
                *html += &format!(r#" {name}="{}""#, escape(&value).replace('"', "&quot;"));
            }
            *html += ">";
            if VOID_ELEMENTS.contains(&tag) {
                return;
            }
            for child in &element.children {
                render_node(child, html);
            }
            *html += &format!("</{tag}>");
        }
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

/// Every word of every vocabulary, for the leak check: rare as they may be, they don't come from a
/// post.
pub(super) fn vocabulary_words() -> Vec<&'static str> {
    vocabulary::all_words()
}

/// Every class name a structure can hold, rendered, and every attribute name.
pub(super) fn class_names() -> Vec<String> {
    class::all_class_names()
}

pub(super) fn attribute_names() -> Vec<&'static str> {
    Attribute::names()
}

/// The class a structure keeps for `class`, as it renders, if it keeps it.
pub(super) fn kept_class(class: &str) -> Option<String> {
    let mut names = Names::for_post("https://post.example/post/");
    class::Class::encode(&mut names, class).map(|class| class.render())
}

/// Whether a host is an embed provider's or a CDN's own, which a structure keeps.
pub(super) fn is_provider_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    matches!(
        host.as_str(),
        "i0.wp.com" | "i1.wp.com" | "i2.wp.com" | "i3.wp.com"
    ) || vocabulary::KnownHost::parse(host.trim_start_matches("www.")).is_some()
}

/// Whether a structure keeps an element by its name, rather than as a `custom-element`.
pub(super) fn is_known_tag(name: &str) -> bool {
    Tag::parse(name).is_some()
}

/// Whether a structure keeps a word in a path, like `embed`.
pub(super) fn is_path_word(word: &str) -> bool {
    vocabulary::PathWord::parse(word).is_some()
}
