//! Getty Images embeds, which the cleanup builds itself.
//!
//! Getty's embed code is a link, an inline script that calls `gie.widgets.load({…})` with the
//! embed's ID and signature, and `widgets.js`, which puts an iframe after the link. The cleanup
//! removes inline scripts, and `widgets.js` throws without one. So this builds the iframe
//! `widgets.js` would, from the inline script, before the cleanup removes it. The page needs no
//! script for Getty at all.
//!
//! Posts the Reader API reads from a site's feed have no scripts left, so their Getty embeds stay
//! links: the signature the iframe needs isn't anywhere else in the post.

use std::collections::HashSet;

use html5ever::{Attribute, LocalName, QualName, ns};
use scraper::{Html, Node, node::Element};

/// Puts the iframe `widgets.js` would build after each Getty link that has an inline script for
/// it.
pub(super) fn build_getty_embeds(document: &mut Html) {
    let embeds: Vec<GettyEmbed> = document
        .tree
        .root()
        .descendants()
        .filter(|node| {
            node.value()
                .as_element()
                .is_some_and(|element| element.name() == "script" && element.attr("src").is_none())
        })
        .flat_map(|script| {
            let text: String = script
                .children()
                .filter_map(|child| child.value().as_text())
                .map(|text| &**text)
                .collect();
            widget_loads(&text)
        })
        .collect();

    let mut built = HashSet::new();
    for embed in embeds {
        if !built.insert(embed.id.clone()) {
            continue;
        }
        let link = document
            .tree
            .root()
            .descendants()
            .find(|node| {
                node.value().as_element().is_some_and(|element| {
                    element.name() == "a" && element.attr("id") == Some(embed.id.as_str())
                })
            })
            .map(|node| node.id());
        if let Some(link) = link {
            document
                .tree
                .get_mut(link)
                .expect("the ID came from this tree")
                .insert_after(Node::Element(embed.iframe()));
        }
    }
}

/// What `widgets.js` builds an embed from: one `gie.widgets.load({…})` call.
#[derive(Debug, PartialEq)]
struct GettyEmbed {
    /// The `id` of the link the embed goes after.
    id: String,
    sig: String,
    /// The images' IDs, separated by commas.
    items: String,
    /// The Getty Images site the embed links to, like `com` or `co.uk`.
    tld: String,
    caption: bool,
    is_360: bool,
    width: Option<u32>,
    height: Option<u32>,
}

impl GettyEmbed {
    /// The embed's fields, if each one `widgets.js` puts in the iframe's URL is safe there without
    /// escaping. `widgets.js` doesn't escape them.
    fn new(fields: &[(String, String)]) -> Option<Self> {
        let field = |name: &str| {
            fields
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.as_str())
        };
        let is_token = |value: &&str, also: &str| {
            !value.is_empty()
                && value.len() <= 100
                && value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || also.contains(c))
        };
        let size = |name: &str| {
            field(name)?
                .trim_end_matches("px")
                .parse::<u32>()
                .ok()
                .filter(|size| *size > 0)
        };
        Some(Self {
            id: field("id").filter(|id| is_token(id, "-_"))?.to_owned(),
            // URL-safe Base64.
            sig: field("sig").filter(|sig| is_token(sig, "-_="))?.to_owned(),
            items: field("items")
                .filter(|items| {
                    items.len() <= 1000
                        && items.split(',').all(|item| {
                            !item.is_empty() && item.bytes().all(|b| b.is_ascii_digit())
                        })
                })?
                .to_owned(),
            tld: field("tld")
                .filter(|tld| is_token(tld, "."))
                .unwrap_or("com")
                .to_ascii_lowercase(),
            caption: field("caption") == Some("true"),
            is_360: field("is360") == Some("true"),
            width: size("w"),
            height: size("h"),
        })
    }

    /// The iframe's URL, the way `widgets.js` makes it.
    fn url(&self) -> String {
        let query = format!(
            "et={}&tld={}&sig={}&caption={}&ver=2",
            self.id, self.tld, self.sig, self.caption
        );
        if self.items.contains(',') {
            format!(
                "https://embed.gettyimages.com/embed?assets={}&{query}",
                self.items
            )
        } else {
            format!("https://embed.gettyimages.com/embed/{}?{query}", self.items)
        }
    }

    /// The iframe `widgets.js` builds, without the wrappers and inline styles it sizes it with: the
    /// cleanup sizes iframes itself.
    fn iframe(&self) -> Element {
        let attribute = |name: &str, value: String| Attribute {
            name: QualName::new(None, ns!(), LocalName::from(name)),
            value: value.into(),
        };
        let mut attributes = vec![
            attribute("src", self.url()),
            attribute("scrolling", "no".to_owned()),
            attribute("frameborder", "0".to_owned()),
        ];
        if let Some(width) = self.width {
            attributes.push(attribute("width", width.to_string()));
        }
        if let Some(height) = self.height {
            attributes.push(attribute("height", height.to_string()));
        }
        if self.is_360 {
            attributes.push(attribute("allowfullscreen", String::new()));
        }
        Element::new(
            QualName::new(None, ns!(html), LocalName::from("iframe")),
            attributes,
        )
    }
}

/// Each `gie.widgets.load({…})` in a script that has what an embed needs.
fn widget_loads(script: &str) -> Vec<GettyEmbed> {
    const CALL: &str = "gie.widgets.load(";
    script
        .match_indices(CALL)
        .filter_map(|(start, _)| object_literal(&script[start + CALL.len()..]))
        .filter_map(|fields| GettyEmbed::new(&fields))
        .collect()
}

/// The fields of the JavaScript object literal `text` starts with, like `{id:'a', caption: true}`,
/// if each value is a quoted string without escapes, or a bare word like `true`.
fn object_literal(text: &str) -> Option<Vec<(String, String)>> {
    let mut rest = text.trim_start().strip_prefix('{')?;
    let mut fields = Vec::new();
    loop {
        rest = rest.trim_start();
        if rest.starts_with('}') {
            return Some(fields);
        }
        let (key, after) = word_or_string(rest)?;
        rest = after.trim_start().strip_prefix(':')?;
        let (value, after) = word_or_string(rest.trim_start())?;
        fields.push((key.to_owned(), value.to_owned()));
        rest = after.trim_start();
        if let Some(after) = rest.strip_prefix(',') {
            rest = after;
        } else if !rest.starts_with('}') {
            return None;
        }
    }
}

/// A quoted string without escapes, or a bare word, at the start of `text`, and what follows it.
fn word_or_string(text: &str) -> Option<(&str, &str)> {
    if let Some(quote) = text.chars().next().filter(|c| *c == '\'' || *c == '"') {
        let inner = &text[1..];
        let end = inner.find(quote)?;
        let value = &inner[..end];
        if value.contains('\\') {
            return None;
        }
        return Some((value, &inner[end + 1..]));
    }
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || "_$.".contains(c)))
        .unwrap_or(text.len());
    (end > 0).then(|| (&text[..end], &text[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::ReaderPostContent;
    use crate::reader::clean::IFRAME_SANDBOX;
    use crate::reader::safety_tests::safety_problems;
    use rstest::rstest;

    /// Getty's embed code, as the Reader API returned it for post 44 on wpmt.co, our WordPress.com
    /// test site. WordPress.com makes a new ID and signature each time it renders the post.
    const EMBED_CODE: &str = r#"<a id='v6GjNmMHRmFpmJdJUX4o_Q' class='gie-single' href='https://www.gettyimages.com/detail/1234567' target='_blank' style='color:#a7a7a7;text-decoration:none;font-weight:normal !important;border:none;display:inline-block;'>Embed from Getty Images</a><script>window.gie=window.gie||function(c){(gie.q=gie.q||[]).push(c)};gie(function(){gie.widgets.load({id:'v6GjNmMHRmFpmJdJUX4o_Q',sig:'0VO_9nekqo1r5vAGaqLSfz0LBHdxgWsdCKmdZJwJHFk=',w:'394px',h:'594px',items:'1234567',caption: false ,tld:'com',is360: false })});</script><script src='//embed-cdn.gettyimages.com/widgets.js' charset='utf-8' async></script>"#;

    #[test]
    fn builds_the_iframe_widgets_js_would() {
        let content = ReaderPostContent::new(EMBED_CODE, None);
        assert_eq!(
            content.html(),
            format!(
                r#"<a class="gie-single" href="https://www.gettyimages.com/detail/1234567" id="v6GjNmMHRmFpmJdJUX4o_Q" target="_blank">Embed from Getty Images</a><iframe frameborder="0" height="594" sandbox="{IFRAME_SANDBOX}" scrolling="no" src="https://embed.gettyimages.com/embed/1234567?et=v6GjNmMHRmFpmJdJUX4o_Q&amp;tld=com&amp;sig=0VO_9nekqo1r5vAGaqLSfz0LBHdxgWsdCKmdZJwJHFk=&amp;caption=false&amp;ver=2" style="width: 100%; max-width: 394px; height: auto; aspect-ratio: 394 / 594;" width="394"></iframe>"#
            )
        );
        // `widgets.js` can only throw without the inline script, so the page doesn't load it.
        assert_eq!(content.scripts(), Vec::<String>::new());
        assert_eq!(safety_problems(&content), Vec::<String>::new());
    }

    #[test]
    fn cleaning_the_result_again_changes_nothing() {
        let html = ReaderPostContent::new(EMBED_CODE, None).html();
        assert_eq!(ReaderPostContent::new(&html, None).html(), html);
    }

    #[rstest]
    #[case::the_embed_code(
        "gie(function(){gie.widgets.load({id:'a_B-1',sig:'c-D_2=',w:'594px',h:'396px',items:'928536434',caption: true ,tld:'in',is360: false })});",
        "https://embed.gettyimages.com/embed/928536434?et=a_B-1&tld=in&sig=c-D_2=&caption=true&ver=2"
    )]
    #[case::several_images(
        "gie.widgets.load({id:'a',sig:'b',items:'1,2,3',caption:false,tld:'co.uk'})",
        "https://embed.gettyimages.com/embed?assets=1,2,3&et=a&tld=co.uk&sig=b&caption=false&ver=2"
    )]
    #[case::double_quotes_and_spaces(
        r#"gie.widgets.load( { "id" : "a" , "sig" : "b" , "items" : "1" } )"#,
        "https://embed.gettyimages.com/embed/1?et=a&tld=com&sig=b&caption=false&ver=2"
    )]
    fn reads_the_embed_from_the_script(#[case] script: &str, #[case] url: &str) {
        let embeds = widget_loads(script);
        assert_eq!(embeds.len(), 1, "{script}");
        assert_eq!(embeds[0].url(), url);
    }

    #[rstest]
    #[case::no_signature("gie.widgets.load({id:'a',items:'1'})")]
    #[case::a_signature_that_would_change_the_url(
        "gie.widgets.load({id:'a',sig:'b&ver=1',items:'1'})"
    )]
    #[case::an_id_that_would_end_the_attribute(r#"gie.widgets.load({id:'a"',sig:'b',items:'1'})"#)]
    #[case::items_that_arent_numbers("gie.widgets.load({id:'a',sig:'b',items:'1/../x'})")]
    #[case::an_escape("gie.widgets.load({id:'a\\'',sig:'b',items:'1'})")]
    #[case::not_an_object_literal("gie.widgets.load(config)")]
    #[case::unfinished("gie.widgets.load({id:'a',sig:'b',items:'1'")]
    fn ignores_an_embed_it_cant_build_safely(#[case] script: &str) {
        assert_eq!(widget_loads(script), Vec::new());
    }

    #[test]
    fn keeps_the_link_when_nothing_has_its_id() {
        let html = EMBED_CODE.replace("<a id='v6GjNmMHRmFpmJdJUX4o_Q'", "<a id='other'");
        let content = ReaderPostContent::new(&html, None);
        assert!(!content.html().contains("<iframe"), "{}", content.html());
    }

    #[test]
    fn marks_a_360_degree_image_for_full_screen() {
        let embeds = widget_loads("gie.widgets.load({id:'a',sig:'b',items:'1',is360: true })");
        let iframe = embeds[0].iframe();
        assert_eq!(iframe.attr("allowfullscreen"), Some(""));
    }
}
