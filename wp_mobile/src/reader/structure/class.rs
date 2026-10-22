//! Classes, by where they come from: a block and its parts, a theme's presets, a block style, an
//! embed's provider, or WordPress's, Jetpack's and embeds' own markup. Any other class goes.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::vocabulary::{
    BlockPart, BlockStyle, CoreBlock, EmbedProvider, JetpackBlock, KeptClass, NumberedClass,
    P2Block, PresetKind, PresetSlug, ProviderPrefix,
};
use super::{AttachmentId, Names, Vocabulary};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Class {
    /// `wp-block-image`.
    Block(Block),
    /// `wp-block-image__crop`, or `wp-block-group-is-layout-flow`.
    Part(Block, BlockPart),
    /// `wp-block-` and a plugin's block, without the plugin's name.
    PluginBlock,
    /// `has-vivid-red-color`.
    Preset(PresetSlug, PresetKind),
    /// `is-style-rounded`.
    Style(BlockStyle),
    /// `is-provider-youtube`.
    Provider(ProviderPrefix, EmbedProvider),
    /// `wp-image-1234`: the image's attachment.
    Attachment(AttachmentId),
    /// `columns-3`.
    Numbered(NumberedClass, u32),
    Kept(KeptClass),
    /// What's left of an element's classes when none of them stay, since whether an element has a
    /// class at all can matter: a plain `<div>` stacks its blocks.
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Block {
    Core(CoreBlock),
    Jetpack(JetpackBlock),
    P2(P2Block),
}

impl Block {
    fn name(self) -> String {
        match self {
            Block::Core(block) => block.as_str().to_owned(),
            Block::Jetpack(block) => format!("jetpack-{}", block.as_str()),
            Block::P2(block) => format!("p2-{}", block.as_str()),
        }
    }

    fn parse(name: &str) -> Option<Block> {
        if let Some(block) = name.strip_prefix("jetpack-").and_then(JetpackBlock::parse) {
            return Some(Block::Jetpack(block));
        }
        if let Some(block) = name.strip_prefix("p2-").and_then(P2Block::parse) {
            return Some(Block::P2(block));
        }
        CoreBlock::parse(name).map(Block::Core)
    }

    /// `wp-block-<block><part>`, with the longest block name that fits.
    fn split(rest: &str) -> Option<(Block, Option<BlockPart>)> {
        if let Some(block) = Block::parse(rest) {
            return Some((block, None));
        }
        rest.char_indices()
            .rev()
            .filter(|(_, c)| matches!(c, '_' | '-'))
            .find_map(|(index, _)| {
                let block = Block::parse(&rest[..index])?;
                let part = BlockPart::parse(&rest[index..])?;
                Some((block, Some(part)))
            })
    }
}

/// A class's numbers of three digits or more, like an image's ID, and its hashes, like a layout's
/// `3a88641f`, as `0`.
fn without_ids(class: &str) -> String {
    class
        .split('-')
        .map(|part| {
            let is_hash = part.len() >= 6
                && part.chars().all(|c| c.is_ascii_hexdigit())
                && part.chars().any(|c| c.is_ascii_digit())
                && part.chars().any(|c| c.is_ascii_alphabetic());
            if is_hash {
                "0".to_owned()
            } else {
                LONG_NUMBER.replace_all(part, "0").into_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("-")
}

impl Class {
    /// The class `class` stands in as, if a structure keeps it.
    pub(crate) fn encode(names: &mut Names, class: &str) -> Option<Class> {
        if let Some(id) = class.strip_prefix("wp-image-")
            && !id.is_empty()
            && id.chars().all(|c| c.is_ascii_digit())
        {
            return Some(Class::Attachment(AttachmentId::of(names, id)));
        }
        let class = without_ids(class);
        if let Some(captures) = PRESET.captures(&class) {
            let slug = PresetSlug::parse(&captures[1]).unwrap_or(PresetSlug::Custom);
            let kind = PresetKind::parse(&captures[2]).expect("the regex has only these");
            return Some(Class::Preset(slug, kind));
        }
        if let Some(style) = class.strip_prefix("is-style-") {
            return Some(Class::Style(
                BlockStyle::parse(style).unwrap_or(BlockStyle::Custom),
            ));
        }
        // An embed's provider, which for another WordPress site is that site's name.
        for prefix in ProviderPrefix::ALL {
            if let Some(provider) = class.strip_prefix(prefix.as_str()) {
                return Some(Class::Provider(
                    *prefix,
                    EmbedProvider::parse(provider).unwrap_or(EmbedProvider::WordpressSite),
                ));
            }
        }
        if let Some((family, number)) = class.rsplit_once('-')
            && let Some(family) = NumberedClass::parse(family)
            && let Ok(number) = number.parse()
        {
            return Some(Class::Numbered(family, number));
        }
        if let Some(rest) = class.strip_prefix("wp-block-") {
            return Some(match Block::split(rest) {
                Some((block, None)) => Class::Block(block),
                Some((block, Some(part))) => Class::Part(block, part),
                None => {
                    let namespace = rest.split(['-', '_']).next().unwrap_or_default();
                    // A Jetpack or P2 block, or a part of one, that the vocabulary doesn't have
                    // isn't a plugin's.
                    if matches!(namespace, "jetpack" | "p2") || Block::parse(namespace).is_some() {
                        return None;
                    }
                    Class::PluginBlock
                }
            });
        }
        KeptClass::parse(&class).map(Class::Kept)
    }

    pub(crate) fn render(self) -> String {
        match self {
            Class::Block(block) => format!("wp-block-{}", block.name()),
            Class::Part(block, part) => format!("wp-block-{}{}", block.name(), part.as_str()),
            Class::PluginBlock => "wp-block-custom".to_owned(),
            Class::Preset(slug, kind) => format!("has-{}{}", slug.as_str(), kind.as_str()),
            Class::Style(style) => format!("is-style-{}", style.as_str()),
            Class::Provider(prefix, provider) => {
                format!("{}{}", prefix.as_str(), provider.as_str())
            }
            Class::Attachment(id) => format!("wp-image-{}", id.0),
            Class::Numbered(family, number) => format!("{}-{number}", family.as_str()),
            Class::Kept(class) => class.as_str().to_owned(),
            Class::Other => "custom".to_owned(),
        }
    }
}

/// Every class a structure can hold that doesn't have a number in it.
pub(crate) fn all_class_names() -> Vec<String> {
    let blocks = CoreBlock::ALL
        .iter()
        .map(|block| Block::Core(*block))
        .chain(JetpackBlock::ALL.iter().map(|block| Block::Jetpack(*block)))
        .chain(P2Block::ALL.iter().map(|block| Block::P2(*block)));
    let mut classes: Vec<Class> = Vec::new();
    for block in blocks {
        classes.push(Class::Block(block));
        classes.extend(BlockPart::ALL.iter().map(|part| Class::Part(block, *part)));
    }
    for slug in PresetSlug::ALL {
        classes.extend(
            PresetKind::ALL
                .iter()
                .map(|kind| Class::Preset(*slug, *kind)),
        );
    }
    classes.extend(BlockStyle::ALL.iter().map(|style| Class::Style(*style)));
    for prefix in ProviderPrefix::ALL {
        classes.extend(
            EmbedProvider::ALL
                .iter()
                .map(|provider| Class::Provider(*prefix, *provider)),
        );
    }
    classes.extend(KeptClass::ALL.iter().map(|class| Class::Kept(*class)));
    classes.push(Class::PluginBlock);
    classes.into_iter().map(Class::render).collect()
}

/// A class for one of a theme's presets, like `has-vivid-red-color`.
static PRESET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^has-([a-z0-9-]+?)(-color|-background-color|-border-color|-font-size|-gradient-background|-font-family)$")
        .expect("a valid regex")
});
static LONG_NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\d{3,}").expect("a valid regex"));
