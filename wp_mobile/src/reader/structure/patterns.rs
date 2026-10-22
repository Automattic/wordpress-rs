//! Finds the patterns in a corpus's structures, and keeps just enough of them to build tests from.
//!
//! This cuts each structure into fragments, and gives each fragment features: what the library and
//! the stylesheet could care about.
//!
//! - Its blocks and elements, and which is inside which, like `wp-block-gallery>wp-block-image`.
//! - Its classes, and the attributes each element has.
//! - What each URL points at: an embed provider or CDN, like `youtube.com`, the post's own site,
//!   another WordPress site, any other site, or an inline image.
//! - Its text's scripts, its shortcodes, a URL in its text, and an ending like `[…]`.
//! - What it holds both ends of: a link to an anchor and the anchor, or Getty's link and the script
//!   the embed is built from. Those are often blocks apart, so a fragment can be a few blocks in a
//!   row.
//!
//! It then picks the fewest, smallest fragments that between them have every feature found in at
//! least two posts. A feature found in only one post is left out, unless it's an embed provider's
//! host, a WordPress or Jetpack block, or the stylesheet or the library's code names it: otherwise
//! it's mostly a plugin's own markup, and it's what could single a post out. It also keeps a few
//! small whole posts, for what only a whole post has: an excerpt, a featured image, an ending.

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt,
    sync::LazyLock,
};

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::attribute::{Attribute, Table};
use super::class::{Block, Class};
use super::text::{Script, Text, Token};
use super::url::{DataUrl, Fragment, Route, Scheme, Site, Url};
use super::vocabulary::{KeptClass, KnownHost, ShortcodeName, Tag};
use super::{Anchor, Comment, Element, Node, Structure, Vocabulary};

/// How many posts a feature has to be in to count, unless the stylesheet or the code names it.
const MIN_POSTS: usize = 2;
/// The largest fragment worth picking for a feature, unless it's a whole top-level block.
const MAX_FRAGMENT: usize = 80;
/// The most top-level blocks in a row a fragment can be.
const MAX_RUN: usize = 4;

/// The patterns: fragments of posts, each as a structure with its post's URL, so links to the post
/// still go to it, and a few whole posts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Patterns {
    pub(crate) fragments: Vec<Structure>,
    pub(crate) posts: Vec<Structure>,
}

impl Patterns {
    pub(crate) fn all(&self) -> impl Iterator<Item = &Structure> {
        self.fragments.iter().chain(&self.posts)
    }
}

// --- Features -------------------------------------------------------------------------------------

/// Something a fragment has that the library or the stylesheet could care about.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Feature {
    Tag(Tag),
    /// A block or element inside another.
    Edge(Identity, Identity),
    Class(Class),
    /// An element with an attribute, by the attribute's name.
    Attribute(Tag, String),
    /// What an attribute's URL points at.
    Url(Tag, String, UrlKind),
    Text(TextFeature),
    Shortcode(ShortcodeName),
    Comment(Comment),
    Relation(Relation),
    Post(PostFeature),
}

/// A block, if an element is one, or its tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Identity {
    Root,
    Block(Block),
    PluginBlock,
    Tag(Tag),
}

/// Where a URL points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum UrlKind {
    Empty,
    Fragment,
    Mailto,
    Tel,
    Sms,
    Javascript,
    Data(DataUrl),
    OtherScheme,
    Unparsed,
    Relative,
    ThisSite,
    WordPressDotCom,
    WordPressCom,
    Files,
    Staging,
    Anywhere,
    Photon,
    Provider(KnownHost),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum TextFeature {
    Script(Script),
    Url,
    EndsWithEllipsis,
}

/// What a fragment holds both ends of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Relation {
    LinkToAnAnchorInThePost,
    GettyLinkAndItsScript,
}

/// What only a whole post has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum PostFeature {
    NoText,
    ExcerptRepeatsStart,
    ExcerptDiffers,
    FeaturedImageInContent,
    FeaturedImageElsewhere,
    CutOff,
}

impl fmt::Display for Identity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Identity::Root => f.write_str("root"),
            Identity::Block(block) => f.write_str(&Class::Block(*block).render()),
            Identity::PluginBlock => f.write_str(&Class::PluginBlock.render()),
            Identity::Tag(tag) => f.write_str(tag.as_str()),
        }
    }
}

impl fmt::Display for UrlKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UrlKind::Data(kind) => write!(f, "data:{kind:?}"),
            UrlKind::Provider(host) => f.write_str(host.as_str()),
            other => write!(f, "{other:?}"),
        }
    }
}

impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Feature::Tag(tag) => write!(f, "tag:{}", tag.as_str()),
            Feature::Edge(parent, child) => write!(f, "edge:{parent}>{child}"),
            Feature::Class(class) => write!(f, "class:{}", class.render()),
            Feature::Attribute(tag, name) => write!(f, "attr:{}[{name}]", tag.as_str()),
            Feature::Url(tag, name, kind) => write!(f, "url:{}[{name}]={kind}", tag.as_str()),
            Feature::Text(TextFeature::Script(script)) => write!(f, "text:{script:?}"),
            Feature::Text(TextFeature::Url) => f.write_str("text:url"),
            Feature::Text(TextFeature::EndsWithEllipsis) => f.write_str("text:ends-with-ellipsis"),
            Feature::Shortcode(name) => write!(f, "shortcode:{}", name.as_str()),
            Feature::Comment(comment) => write!(f, "comment:{comment:?}"),
            Feature::Relation(relation) => write!(f, "relation:{relation:?}"),
            Feature::Post(feature) => write!(f, "post:{feature:?}"),
        }
    }
}

fn identity(element: &Element) -> Identity {
    classes(element)
        .find_map(|class| match class {
            Class::Block(block) => Some(Identity::Block(*block)),
            Class::PluginBlock => Some(Identity::PluginBlock),
            _ => None,
        })
        .unwrap_or(Identity::Tag(element.tag))
}

fn classes(element: &Element) -> impl Iterator<Item = &Class> {
    element
        .attributes
        .iter()
        .filter_map(|attribute| match attribute {
            Attribute::Class(classes) => Some(classes.classes()),
            _ => None,
        })
        .flatten()
}

fn url_kind(url: &Url) -> UrlKind {
    let site = |scheme: Scheme, site: &Site| match (scheme, site) {
        (Scheme::Relative, _) => UrlKind::Relative,
        (_, Site::This { .. }) => UrlKind::ThisSite,
        (_, Site::WordPressDotCom { .. }) => UrlKind::WordPressDotCom,
        (_, Site::WordPressCom(_)) => UrlKind::WordPressCom,
        (_, Site::Files(_)) => UrlKind::Files,
        (_, Site::Staging(_)) => UrlKind::Staging,
        (_, Site::Anywhere) => UrlKind::Anywhere,
    };
    match url {
        Url::Empty => UrlKind::Empty,
        Url::Anchor(_) | Url::Hash => UrlKind::Fragment,
        Url::Mailto => UrlKind::Mailto,
        Url::Tel => UrlKind::Tel,
        Url::Sms => UrlKind::Sms,
        Url::Javascript => UrlKind::Javascript,
        Url::Data(kind, _) => UrlKind::Data(*kind),
        Url::OtherScheme(_) => UrlKind::OtherScheme,
        Url::Unparsed => UrlKind::Unparsed,
        Url::Page {
            scheme, site: at, ..
        }
        | Url::Image {
            scheme, site: at, ..
        }
        | Url::File {
            scheme, site: at, ..
        } => site(*scheme, at),
        Url::Photon { .. } => UrlKind::Photon,
        Url::Provider { host, .. } => UrlKind::Provider(*host),
        Url::Anywhere { .. } => UrlKind::Anywhere,
    }
}

static ELLIPSIS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\[…\]|\[\.\.\.\]|…)\s*$").expect("a valid regex"));

fn text_features(text: &Text) -> BTreeSet<Feature> {
    let mut found = BTreeSet::new();
    for token in &text.0 {
        match token {
            Token::Script { script, .. } => {
                found.insert(Feature::Text(TextFeature::Script(*script)));
            }
            Token::Unspaced { kana, .. } => {
                let script = if *kana { Script::Hiragana } else { Script::Han };
                found.insert(Feature::Text(TextFeature::Script(script)));
            }
            Token::Url(_) => {
                found.insert(Feature::Text(TextFeature::Url));
            }
            Token::Shortcode(shortcode) => {
                found.insert(Feature::Shortcode(shortcode.name));
            }
            _ => {}
        }
    }
    if ELLIPSIS.is_match(&text.render()) {
        found.insert(Feature::Text(TextFeature::EndsWithEllipsis));
    }
    found
}

/// A node's features, and its descendants'.
pub(crate) fn features(node: &Node, parent: Identity) -> BTreeSet<Feature> {
    match node {
        Node::Text(text) => text_features(text),
        Node::Comment(comment) => BTreeSet::from([Feature::Comment(*comment)]),
        Node::Getty(_) => BTreeSet::new(),
        Node::Element(element) => {
            let me = identity(element);
            let mut found = BTreeSet::from([Feature::Tag(element.tag), Feature::Edge(parent, me)]);
            found.extend(classes(element).map(|class| Feature::Class(normalized(*class))));
            for attribute in &element.attributes {
                if matches!(attribute, Attribute::Class(_)) {
                    continue;
                }
                let name = attribute.render().0.into_owned();
                if let Some(url) = attribute.urls().first() {
                    found.insert(Feature::Url(element.tag, name.clone(), url_kind(url)));
                }
                found.insert(Feature::Attribute(element.tag, name));
            }
            for child in &element.children {
                found.extend(features(child, me));
            }
            found
        }
    }
}

/// A class as a feature: every image's attachment is the same feature.
fn normalized(class: Class) -> Class {
    match class {
        Class::Attachment(_) => Class::Attachment(super::AttachmentId(0)),
        class => class,
    }
}

/// Every element in a node, the node first.
fn elements(node: &Node) -> Vec<&Element> {
    let mut found = Vec::new();
    if let Node::Element(element) = node {
        found.push(element);
        for child in &element.children {
            found.extend(elements(child));
        }
    }
    found
}

/// What the nodes hold both ends of.
fn relations(nodes: &[&Node]) -> BTreeSet<Feature> {
    let (mut anchors, mut targets) = (HashSet::new(), HashSet::new());
    let (mut getty_links, mut getty_loads) = (HashSet::new(), HashSet::new());
    let mut collect = |node: &Node| {
        if let Node::Getty(loads) = node {
            getty_loads.extend(loads.iter().flat_map(|load| load.ids()));
        }
    };
    for node in nodes {
        walk(node, &mut collect);
        for element in elements(node) {
            let is_getty_link = element.tag == Tag::A
                && classes(element).any(|class| *class == Class::Kept(KeptClass::GieSingle));
            for attribute in &element.attributes {
                match attribute {
                    // Headings and footnotes are anchors of their own kinds, which other features
                    // cover.
                    Attribute::Id(ids) if !is_heading(element.tag) => {
                        anchors.extend(ids.anchors().iter().copied());
                        if is_getty_link {
                            getty_links.extend(ids.anchors().iter().copied());
                        }
                    }
                    Attribute::Id(ids) if is_getty_link => {
                        getty_links.extend(ids.anchors().iter().copied());
                    }
                    // A link within the post: `#a1`, or the post's own URL with `#a1`.
                    Attribute::Href(url) => targets.extend(in_page_target(url)),
                    _ => {}
                }
            }
        }
    }
    let mut found = BTreeSet::new();
    if !targets.is_disjoint(&anchors) {
        found.insert(Feature::Relation(Relation::LinkToAnAnchorInThePost));
    }
    if !getty_links.is_disjoint(&getty_loads) {
        found.insert(Feature::Relation(Relation::GettyLinkAndItsScript));
    }
    found
}

fn walk(node: &Node, visit: &mut impl FnMut(&Node)) {
    visit(node);
    if let Node::Element(element) = node {
        for child in &element.children {
            walk(child, visit);
        }
    }
}

/// Every node in a node, in order, not the node itself.
fn descendants(node: &Node) -> Vec<&Node> {
    let mut found = Vec::new();
    if let Node::Element(element) = node {
        for child in &element.children {
            found.push(child);
            found.extend(descendants(child));
        }
    }
    found
}

fn is_heading(tag: Tag) -> bool {
    matches!(
        tag,
        Tag::H1 | Tag::H2 | Tag::H3 | Tag::H4 | Tag::H5 | Tag::H6
    )
}

fn in_page_target(url: &Url) -> Option<Anchor> {
    match url {
        Url::Anchor(anchor) => Some(*anchor),
        Url::Page {
            path,
            fragment: Some(Fragment::Anchor(anchor)),
            ..
        } if matches!(path.route, Route::ThisPost(_)) => Some(*anchor),
        _ => None,
    }
}

/// How many nodes a node is, its own included.
fn size(node: &Node) -> usize {
    match node {
        Node::Element(element) => 1 + element.children.iter().map(size).sum::<usize>(),
        _ => 1,
    }
}

/// The text the nodes hold, without scripts' and styles'.
fn plain_text(nodes: &[Node]) -> String {
    let mut text = String::new();
    for node in nodes {
        match node {
            Node::Text(found) => text += &found.render(),
            Node::Element(element) if !matches!(element.tag, Tag::Script | Tag::Style) => {
                text += &plain_text(&element.children);
            }
            _ => {}
        }
    }
    text
}

/// A URL without its size, so the same image at two sizes matches.
fn image_key(url: &Url) -> String {
    static SIZE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"-\d+x\d+(\.\w+)$").expect("a valid regex"));
    let rendered = url.render();
    let path = rendered.split('?').next().unwrap_or_default();
    let path = SIZE.replace(path, "$1");
    path.split_once("//")
        .map_or(path.as_ref(), |(_, rest)| rest)
        .to_owned()
}

fn post_features(structure: &Structure) -> BTreeSet<Feature> {
    let mut found = BTreeSet::new();
    let without_spaces = |text: String| -> String { text.split_whitespace().collect() };
    let text = without_spaces(plain_text(&structure.content));
    let excerpt = without_spaces(plain_text(&structure.excerpt));
    if text.is_empty() {
        found.insert(Feature::Post(PostFeature::NoText));
    }
    if !excerpt.is_empty() {
        let start: String = excerpt.chars().take(40).collect();
        found.insert(Feature::Post(if text.starts_with(&start) {
            PostFeature::ExcerptRepeatsStart
        } else {
            PostFeature::ExcerptDiffers
        }));
    }
    if structure.featured_image != Url::Empty {
        let images: HashSet<String> = structure
            .content
            .iter()
            .flat_map(elements)
            .filter(|element| element.tag == Tag::Img)
            .flat_map(|element| &element.attributes)
            .filter_map(|attribute| match attribute {
                Attribute::Src(url) => Some(image_key(url)),
                _ => None,
            })
            .collect();
        found.insert(Feature::Post(
            if images.contains(&image_key(&structure.featured_image)) {
                PostFeature::FeaturedImageInContent
            } else {
                PostFeature::FeaturedImageElsewhere
            },
        ));
    }
    let more_link =
        structure.content.iter().flat_map(elements).any(|element| {
            classes(element).any(|class| *class == Class::Kept(KeptClass::MoreLink))
        });
    if more_link || ELLIPSIS.is_match(plain_text(&structure.content).trim_end()) {
        found.insert(Feature::Post(PostFeature::CutOff));
    }
    found
}

// --- Picking --------------------------------------------------------------------------------------

/// Something that could be picked: some nodes of one post, or a whole post.
struct Candidate<'a> {
    post: usize,
    nodes: Vec<&'a Node>,
    /// Its features, by number.
    features: Vec<u32>,
    size: usize,
}

/// The fewest, smallest candidates that have every feature in `universe`, greedily: each pick has
/// the most features left for its size. Each pick comes with the features it was picked for, and
/// the features no candidate has come last.
fn cover(candidates: &[Candidate], universe: &HashSet<u32>) -> (Vec<(usize, Vec<u32>)>, Vec<u32>) {
    let mut uncovered = universe.clone();
    let mut alive: Vec<usize> = (0..candidates.len()).collect();
    let mut picked = Vec::new();
    while !uncovered.is_empty() {
        let gain = |index: usize| {
            candidates[index]
                .features
                .iter()
                .filter(|feature| uncovered.contains(feature))
                .count()
        };
        // A candidate with nothing left to give can't gain anything later.
        alive.retain(|index| gain(*index) > 0);
        let Some(best) = alive.iter().copied().reduce(|best, index| {
            let score =
                |index: usize| gain(index) as f64 / (candidates[index].size.max(1) as f64).sqrt();
            if score(index) > score(best) {
                index
            } else {
                best
            }
        }) else {
            break;
        };
        let gained: Vec<u32> = candidates[best]
            .features
            .iter()
            .copied()
            .filter(|feature| uncovered.contains(feature))
            .collect();
        for feature in &gained {
            uncovered.remove(feature);
        }
        picked.push((best, gained));
    }
    let mut missed: Vec<u32> = uncovered.into_iter().collect();
    missed.sort_unstable();
    (picked, missed)
}

/// What the stylesheet and the library's code read, so their features count however rare.
pub(crate) struct Named {
    pub(crate) classes: BTreeSet<String>,
    pub(crate) attributes: BTreeSet<String>,
}

/// The patterns, and what each was picked for.
pub(crate) struct Picked {
    pub(crate) patterns: Patterns,
    /// What each fragment, then each whole post, was picked for.
    pub(crate) covers: Vec<Vec<Feature>>,
    pub(crate) report: Vec<String>,
}

/// Picks the patterns in `structures`.
pub(crate) fn pick(structures: &[Structure], named: &Named) -> Picked {
    let mut ids: HashMap<Feature, u32> = HashMap::new();
    let mut by_id: Vec<Feature> = Vec::new();
    let mut id = |feature: Feature, by_id: &mut Vec<Feature>| -> u32 {
        *ids.entry(feature.clone()).or_insert_with(|| {
            by_id.push(feature);
            (by_id.len() - 1) as u32
        })
    };
    let mut posts_with: HashMap<u32, BTreeSet<usize>> = HashMap::new();
    let mut fragments: Vec<Candidate> = Vec::new();
    let mut numbered = |features: BTreeSet<Feature>, by_id: &mut Vec<Feature>| -> Vec<u32> {
        let mut numbers: Vec<u32> = features
            .into_iter()
            .map(|feature| id(feature, by_id))
            .collect();
        numbers.sort_unstable();
        numbers
    };

    for (index, structure) in structures.iter().enumerate() {
        let tops: Vec<&Node> = structure
            .content
            .iter()
            .filter(|node| !matches!(node, Node::Text(text) if text.render().trim().is_empty()))
            .collect();
        for feature in numbered(relations(&tops), &mut by_id) {
            posts_with.entry(feature).or_default().insert(index);
        }
        for start in 0..tops.len() {
            for length in 2..=MAX_RUN {
                let Some(run) = tops.get(start..start + length) else {
                    break;
                };
                let found = relations(run);
                if !found.is_empty() {
                    let mut all = found;
                    for node in run {
                        all.extend(features(node, Identity::Root));
                    }
                    fragments.push(Candidate {
                        post: index,
                        nodes: run.to_vec(),
                        features: numbered(all, &mut by_id),
                        size: run.iter().map(|node| size(node)).sum(),
                    });
                }
            }
        }
        for top in &tops {
            let top_features = numbered(features(top, Identity::Root), &mut by_id);
            for feature in &top_features {
                posts_with.entry(*feature).or_default().insert(index);
            }
            let mut with_relations = top_features;
            with_relations.extend(numbered(relations(&[*top]), &mut by_id));
            with_relations.sort_unstable();
            with_relations.dedup();
            fragments.push(Candidate {
                post: index,
                nodes: vec![*top],
                features: with_relations,
                size: size(top),
            });
            for node in descendants(top) {
                if !matches!(node, Node::Element(_)) {
                    continue;
                }
                let n = size(node);
                if n <= MAX_FRAGMENT {
                    let mut found = features(node, Identity::Root);
                    found.extend(relations(&[node]));
                    fragments.push(Candidate {
                        post: index,
                        nodes: vec![node],
                        features: numbered(found, &mut by_id),
                        size: n,
                    });
                }
            }
        }
        let content: Vec<&Node> = structure.content.iter().collect();
        let mut whole = post_features(structure);
        whole.extend(relations(&content));
        for feature in numbered(whole, &mut by_id) {
            posts_with.entry(feature).or_default().insert(index);
        }
    }

    let wanted = |feature: &Feature, posts: usize| -> bool {
        if posts >= MIN_POSTS {
            return true;
        }
        match feature {
            Feature::Url(_, _, kind) => matches!(kind, UrlKind::Provider(_) | UrlKind::Photon),
            Feature::Class(class) => {
                let class = class.render();
                (class.starts_with("wp-block-") && class != "wp-block-custom")
                    || named.classes.contains(&class)
            }
            Feature::Attribute(_, name) => named.attributes.contains(name),
            Feature::Edge(..) => false,
            _ => true,
        }
    };
    let universe: HashSet<u32> = posts_with
        .iter()
        .filter(|(feature, posts)| wanted(&by_id[**feature as usize], posts.len()))
        .map(|(feature, _)| *feature)
        .collect();
    let fragment_universe: HashSet<u32> = universe
        .iter()
        .copied()
        .filter(|feature| !matches!(by_id[*feature as usize], Feature::Post(_)))
        .collect();
    let (picked, missed) = cover(&fragments, &fragment_universe);

    // Whole posts, for what only a whole post has, and for what no run of blocks could hold.
    let whole_posts: Vec<Candidate> = structures
        .iter()
        .enumerate()
        .map(|(index, structure)| {
            let content: Vec<&Node> = structure.content.iter().collect();
            let mut found = post_features(structure);
            found.extend(relations(&content));
            Candidate {
                post: index,
                nodes: content.clone(),
                features: numbered(found, &mut by_id),
                size: content.iter().map(|node| size(node)).sum(),
            }
        })
        .collect();
    let mut post_universe: HashSet<u32> = universe
        .iter()
        .copied()
        .filter(|feature| matches!(by_id[*feature as usize], Feature::Post(_)))
        .collect();
    post_universe.extend(missed);
    let (picked_posts, missed_posts) = cover(&whole_posts, &post_universe);

    let names = |features: &[u32]| -> Vec<Feature> {
        let mut named: Vec<Feature> = features
            .iter()
            .map(|feature| by_id[*feature as usize].clone())
            .collect();
        named.sort();
        named
    };
    let patterns = Patterns {
        fragments: picked
            .iter()
            .map(|(index, _)| {
                let candidate = &fragments[*index];
                Structure {
                    url: structures[candidate.post].url.clone(),
                    featured_image: Url::Empty,
                    content: candidate.nodes.iter().map(|node| (*node).clone()).collect(),
                    excerpt: Vec::new(),
                }
            })
            .collect(),
        posts: picked_posts
            .iter()
            .map(|(index, _)| structures[whole_posts[*index].post].clone())
            .collect(),
    };
    let covers = picked
        .iter()
        .chain(&picked_posts)
        .map(|(_, gained)| names(gained))
        .collect();

    let sizes: Vec<usize> = picked
        .iter()
        .map(|(index, _)| fragments[*index].size)
        .collect();
    let mut sorted = sizes.clone();
    sorted.sort_unstable();
    let mut report = vec![
        format!(
            "{} posts, {} fragments, {} features, {} of them in at least {MIN_POSTS} posts or named \
             by the code",
            structures.len(),
            fragments.len(),
            posts_with.len(),
            universe.len()
        ),
        format!(
            "{} fragments cover them: {} elements, median {}, largest {}",
            picked.len(),
            sizes.iter().sum::<usize>(),
            sorted.get(sorted.len() / 2).copied().unwrap_or_default(),
            sorted.last().copied().unwrap_or_default()
        ),
        format!(
            "{} whole posts cover {:?}",
            picked_posts.len(),
            names(&post_universe.iter().copied().collect::<Vec<_>>())
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        ),
    ];
    if !missed_posts.is_empty() {
        report.push(format!(
            "Not covered: {:?}",
            names(&missed_posts)
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        ));
    }
    Picked {
        patterns,
        covers,
        report,
    }
}
