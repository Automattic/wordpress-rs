use std::sync::{Arc, LazyLock};

use scraper::{ElementRef, Html, Node, Selector};
use url::Url;

use super::anchors::{ReaderPostAnchor, anchors};
use super::clean;
use super::gists::{gist_data_urls, remove_gist_data};
use super::images::{
    ReaderPostImage, image_key, image_urls_to_preload, parse_dimension, parse_size, parse_srcset,
    photon_sizes, size_from_query,
};
use super::page::{ReaderPageOptions, page_html};
use super::shortcodes::hide_shortcodes;
use super::text::{
    excerpt_source_text, is_cut_off, is_manual_excerpt, plain_text, reading_time_minutes,
    simplified_html,
};

/// A post from the Reader API, cleaned up for the Reader's post page, with what the app might want
/// to know about it.
#[derive(Debug, uniffi::Object)]
pub struct ReaderPostContent {
    html: String,
    /// The addresses of the gists' data, which `reader.js` loads.
    gist_data: Vec<String>,
    offline_html: String,
    offline_image_urls: Vec<String>,
    scripts: Vec<String>,
    images: Vec<ReaderPostImage>,
    galleries: Vec<ReaderPostGallery>,
    mentions: Vec<ReaderPostMention>,
    anchors: Vec<ReaderPostAnchor>,
    plain_text: String,
    excerpt_source_text: String,
    simplified_html: String,
    is_cut_off: bool,
}

/// A gallery in a Reader post.
#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Record)]
pub struct ReaderPostGallery {
    pub images: Vec<ReaderPostImage>,
}

/// Where an image is in one of a Reader post's galleries.
#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Record)]
pub struct ReaderPostGalleryPosition {
    /// The gallery, for paging through in an image viewer.
    pub gallery: ReaderPostGallery,
    /// Which of the gallery's images it is, from 0.
    pub index: u32,
}

/// Someone a Reader post mentions, like `@username` on a P2.
#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Record)]
pub struct ReaderPostMention {
    pub username: String,
    /// Where the mention links to.
    pub url: String,
}

#[uniffi::export]
impl ReaderPostContent {
    /// Cleans up a post's HTML from the Reader API, and finds what's in it. `url` is the post's
    /// address, so links and images relative to it still work on the Reader's page.
    ///
    /// The cleanup:
    ///
    /// - Removes inline `style` attributes and `<style>` elements, so the Reader's own stylesheet
    ///   decides how the post looks.
    /// - Removes every `<script>`, and lists the scripts the post's embeds need in
    ///   [`scripts`](Self::scripts) instead.
    /// - Removes HTML comments, `<noscript>` elements, and any paragraph that held only comments.
    /// - Makes URLs work from the Reader's page: protocol-relative URLs (`//example.com`) get
    ///   `https:`, and URLs relative to the post are resolved against `url`.
    /// - Loads images and iframes over HTTPS, even when the post has `http:` URLs for them. Links
    ///   keep their URLs.
    /// - Removes iframes that don't load a web page, and iframes' `srcdoc`, which would run as part
    ///   of the page.
    /// - Stops iframes from navigating the whole page, which would replace the post with a page of
    ///   the embed's own. Each gets a `sandbox` that allows everything else, unless it has one.
    /// - Removes anything else that would run as part of the page: event handlers like `onclick`,
    ///   URLs that run something, like `javascript:`, on any element, SVG animations that change a
    ///   link, and `<base>`, `<link>` and `<meta>` elements other than microdata. Links can go to
    ///   web pages, email addresses and phone numbers; images and media can also be `data:` URLs.
    /// - Cleans the post the way the web view will parse it. Some markup parses into a tree its own
    ///   HTML doesn't parse back into, like a `<form>` inside a `<form>`, so the cleanup parses its
    ///   result again until nothing changes. A post that keeps changing keeps only its text.
    /// - Loads lazily-loaded images: an image whose `src` is a placeholder gets the URL a
    ///   lazy-loading plugin left in `data-src` or the like, for a script the page doesn't run.
    /// - Turns links to somewhere in the post that use the post's full address, like
    ///   `https://example.com/post/#notes`, into `#notes`, so they stay in the page.
    /// - Stops `<video>` and `<audio>` from playing on their own, and gives them controls so they
    ///   can still be played. A cover block's background video, which the Reader API turns into a
    ///   `<div>`, becomes a `<video>` again.
    /// - Turns an embed that never loaded, which only holds its URL, into a link.
    /// - Builds Getty Images embeds from the inline script in their embed code, which the page
    ///   doesn't run: the iframe Getty's own script would add after the link.
    /// - Replaces GitHub gists with their code, in a code block linked to the gist. On the
    ///   [page](Self::page_html), the shared script loads the gist from GitHub, and shows it in the
    ///   code block's place. Offline, or when GitHub doesn't answer, the code block stays.
    /// - Hides shortcodes the Reader API sent without running, like `[gallery ids="1,2"]`, keeping
    ///   what they enclose. An `[embed]` of a URL becomes a link to it.
    /// - Replaces emoji images with the emoji, so they show in the platform's own style, and
    ///   offline.
    ///
    /// The HTML is re-serialized, so it also differs from the input in ways that don't change how
    /// it renders: attributes come out in alphabetical order, character references like `&#8217;`
    /// come out as the characters themselves (so the page must be UTF-8), and `<br />` comes out as
    /// `<br>`.
    #[uniffi::constructor(default(url = None))]
    pub fn new(html: &str, url: Option<String>) -> Arc<Self> {
        let post_url = url.as_deref().and_then(|url| Url::parse(url).ok());
        let mut document = Html::parse_fragment(html);
        let mut scripts = clean::clean(&mut document, post_url.as_ref());
        // WordPress leaves shortcodes out of the excerpts it makes, along with what they enclose,
        // so the excerpt check needs them.
        let excerpt_source_text = excerpt_source_text(document.root_element());
        hide_shortcodes(&mut document);
        let document = clean::reparse(
            document,
            &mut scripts,
            post_url.as_ref(),
            clean::REPARSE_ROUNDS,
        );
        Self::from_cleaned(document, scripts, excerpt_source_text)
    }

    /// The post, with smaller sizes of each image that only comes in one size, so the web view can
    /// load the size that fits the screen. Photon, WordPress.com's image CDN, makes them.
    ///
    /// Only use it for posts from public sites, whose `site_is_private` is `false` in the Reader API:
    /// Photon can't fetch a private site's images.
    pub fn with_photon_sizes(&self) -> Arc<Self> {
        let mut document = Html::parse_fragment(&self.html);

        let srcsets: Vec<_> = document
            .root_element()
            .select(&IMAGES)
            .filter(|img| img.value().attr("srcset").is_none_or(str::is_empty))
            .filter_map(|img| {
                let image = image(img)?;
                let sizes = photon_sizes(&image.url, image.width);
                let srcset = sizes
                    .iter()
                    .map(|size| format!("{} {}w", size.url, size.width))
                    .collect::<Vec<_>>()
                    .join(", ");
                (!sizes.is_empty()).then(|| (img.id(), srcset))
            })
            .collect();
        for (id, srcset) in srcsets {
            if let Some(mut node) = document.tree.get_mut(id)
                && let Node::Element(element) = node.value()
            {
                clean::set_attribute(element, "srcset", &srcset);
            }
        }

        Self::from_cleaned(
            document,
            self.scripts.clone(),
            self.excerpt_source_text.clone(),
        )
    }

    /// The post, as an HTML fragment for the page's `<body>`.
    pub fn html(&self) -> String {
        self.html.clone()
    }

    /// The post for reading offline, as an HTML fragment for the page's `<body>`.
    ///
    /// It's [`html`](Self::html), except that each image only offers the web view its `src`, rather
    /// than a choice of sizes. Download [`offline_image_urls`](Self::offline_image_urls) ahead of
    /// time, and the post's images show without a connection.
    pub fn offline_html(&self) -> String {
        self.offline_html.clone()
    }

    /// Every image [`offline_html`](Self::offline_html) shows, emoji included, for downloading
    /// ahead of time.
    pub fn offline_image_urls(&self) -> Vec<String> {
        self.offline_image_urls.clone()
    }

    /// The scripts the post needs, in the order the page should load them.
    ///
    /// The cleanup removes every `<script>` from the post, so the page loads these instead. They're
    /// URLs, not HTML – escape them before writing them into a `src` attribute.
    ///
    /// Gists aren't here: the shared script loads them.
    pub fn scripts(&self) -> Vec<String> {
        self.scripts.clone()
    }

    /// The post's images, in order, apart from emoji.
    pub fn images(&self) -> Vec<ReaderPostImage> {
        self.images.clone()
    }

    /// The post's galleries, in order, for paging through in an image viewer. Their images are in
    /// [`images`](Self::images) too.
    pub fn galleries(&self) -> Vec<ReaderPostGallery> {
        self.galleries.clone()
    }

    /// The people the post mentions, in the order it first mentions them.
    pub fn mentions(&self) -> Vec<ReaderPostMention> {
        self.mentions.clone()
    }

    /// The places in the post that links can go to: headings with an `id`, footnotes, and anywhere
    /// else a link in the post goes to. In order.
    ///
    /// When the reader taps a link to one, the page's script tells the app which, so the app can
    /// scroll there, or show a footnote's text where the reader is.
    pub fn anchors(&self) -> Vec<ReaderPostAnchor> {
        self.anchors.clone()
    }

    /// The post's text, with a line break between blocks like paragraphs and list items.
    pub fn plain_text(&self) -> String {
        self.plain_text.clone()
    }

    /// The post's headings, paragraphs and other text, one element to a line, with images' text
    /// alternatives, and none of the markup in between. It's for language models: summaries and
    /// translation. The text isn't escaped, so it isn't for a web view.
    pub fn simplified_html(&self) -> String {
        self.simplified_html.clone()
    }

    /// The page for the post, ready for the app's web view: the post, the theme's colours, the
    /// shared stylesheet and script, the app's own CSS, and the scripts the post's embeds need.
    ///
    /// For smaller images from Photon, call it on [`with_photon_sizes`](Self::with_photon_sizes)'s
    /// result.
    pub fn page_html(&self, options: ReaderPageOptions) -> String {
        let html = if options.offline {
            &self.offline_html
        } else {
            &self.html
        };
        page_html(
            html,
            &self.plain_text,
            &self.scripts,
            &self.gist_data,
            options,
        )
    }

    /// Whether `excerpt` is one the author wrote, rather than one WordPress made by shortening the
    /// post. The Reader only shows the author's. `excerpt` can be text or HTML.
    pub fn is_manual_excerpt(&self, excerpt: &str) -> bool {
        is_manual_excerpt(excerpt, &self.excerpt_source_text)
    }

    /// Whether the post shows an image anywhere, at any size. Use it to avoid showing a featured
    /// image the post already shows.
    pub fn includes_image(&self, url: &str) -> bool {
        let Some(key) = image_key(url) else {
            return false;
        };
        self.images
            .iter()
            .flat_map(ReaderPostImage::urls)
            .any(|url| image_key(url).as_ref() == Some(&key))
    }

    /// Which gallery an image the reader tapped is in, and where, so the image viewer can page
    /// through the gallery from it. `url` is any URL for the image, at any size: the `image:`
    /// message's, or a link's. `None` if it isn't in a gallery.
    pub fn gallery_position(&self, url: &str) -> Option<ReaderPostGalleryPosition> {
        let key = image_key(url)?;
        self.galleries.iter().find_map(|gallery| {
            let index = gallery.images.iter().position(|image| {
                image
                    .urls()
                    .any(|url| image_key(url).as_ref() == Some(&key))
            })?;
            Some(ReaderPostGalleryPosition {
                gallery: gallery.clone(),
                index: index as u32,
            })
        })
    }

    /// How long the post takes to read, in whole minutes. `None` for a post without text.
    ///
    /// Don't show it for a post that's only the start of the real one: one that
    /// [`is_cut_off`](Self::is_cut_off), or that the Reader API says to show an excerpt of
    /// (`use_excerpt`).
    pub fn reading_time_minutes(&self) -> Option<u32> {
        reading_time_minutes(&self.plain_text)
    }

    /// Whether the post is only the start of the real one, the way some sites' feeds send it: it
    /// ends with a "Continue reading" link, or with `[…]`. The Reader can offer the whole post on
    /// the web.
    pub fn is_cut_off(&self) -> bool {
        self.is_cut_off
    }

    /// The URLs worth downloading ahead of time, so the post shows at once when the reader opens it:
    /// its images, at the size the page would load in a web view `width` points wide with `scale`
    /// pixels to a point, video posters, and the scripts its embeds need.
    ///
    /// The web view picks images' sizes itself, and this matches it for images as wide as the
    /// screen. To read offline, use [`offline_image_urls`](Self::offline_image_urls) instead.
    pub fn preload_urls(&self, width: u32, scale: f64) -> Vec<String> {
        let document = Html::parse_fragment(&self.html);
        let mut urls = image_urls_to_preload(document.root_element(), width, scale);
        urls.extend(self.scripts.iter().cloned());
        urls
    }
}

impl ReaderPostContent {
    /// Finds what's in a post that's been through the cleanup.
    fn from_cleaned(
        mut document: Html,
        scripts: Vec<String>,
        excerpt_source_text: String,
    ) -> Arc<Self> {
        let root = document.root_element();
        let images = root.select(&IMAGES).filter_map(image).collect();
        let galleries = galleries(root);
        let mentions = mentions(root);
        let anchors = anchors(root);
        let plain_text = plain_text(root);
        let simplified_html = simplified_html(root);
        let is_cut_off = is_cut_off(root);
        let html = root.inner_html();
        let gist_data = gist_data_urls(root);

        show_only_image_urls(&mut document);
        remove_gist_data(&mut document);
        let offline_root = document.root_element();
        let mut offline_image_urls: Vec<String> = Vec::new();
        for src in offline_root
            .select(&ALL_IMAGES)
            .filter_map(|img| img.value().attr("src"))
        {
            if is_image_url(src) && !offline_image_urls.iter().any(|url| url == src) {
                offline_image_urls.push(src.to_owned());
            }
        }
        let offline_html = offline_root.inner_html();

        Arc::new(Self {
            html,
            gist_data,
            offline_html,
            offline_image_urls,
            scripts,
            images,
            galleries,
            mentions,
            anchors,
            plain_text,
            excerpt_source_text,
            simplified_html,
            is_cut_off,
        })
    }
}

fn selector(selectors: &str) -> Selector {
    Selector::parse(selectors).expect("the selector is valid")
}

static IMAGES: LazyLock<Selector> = LazyLock::new(|| selector("img:not(.emoji, .wp-smiley)"));
static ALL_IMAGES: LazyLock<Selector> = LazyLock::new(|| selector("img"));

// Gallery blocks have been `<figure>`, `<div>` and `<ul>` over the years. `div.gallery` is the
// classic editor's `[gallery]`.
static GALLERIES: LazyLock<Selector> = LazyLock::new(|| {
    selector(
        ".wp-block-gallery, .blocks-gallery-grid, .wp-block-jetpack-tiled-gallery, .tiled-gallery, \
         div.gallery, .wp-block-jetpack-slideshow",
    )
});

static MENTIONS: LazyLock<Selector> = LazyLock::new(|| selector("a.mention"));

/// Whether an image's `src` could be an image. Some Jetpack sites send just their address, like
/// `https://example.com`, which is a page.
fn is_image_url(src: &str) -> bool {
    match Url::parse(src) {
        Ok(url) => url.path() != "/",
        Err(_) => !src.trim().is_empty(),
    }
}

fn image(img: ElementRef) -> Option<ReaderPostImage> {
    let element = img.value();
    let url = element
        .attr("src")
        .filter(|src| is_image_url(src))?
        .to_owned();
    let (width, height) = match element.attr("data-orig-size").and_then(parse_size) {
        Some((width, height)) => (Some(width), Some(height)),
        None => {
            let (query_width, query_height) = size_from_query(&url);
            (
                element
                    .attr("width")
                    .and_then(parse_dimension)
                    .or(query_width),
                element
                    .attr("height")
                    .and_then(parse_dimension)
                    .or(query_height),
            )
        }
    };
    let html_text = |html: &str| text(Html::parse_fragment(html).root_element());
    let caption = element
        .attr("data-image-caption")
        .and_then(html_text)
        .or_else(|| figure_caption(img));
    let link = img
        .ancestors()
        .filter_map(ElementRef::wrap)
        .find(|ancestor| ancestor.value().name() == "a")
        .and_then(|link| link.value().attr("href"))
        .map(str::to_owned);

    Some(ReaderPostImage {
        url,
        sizes: element.attr("srcset").map(parse_srcset).unwrap_or_default(),
        original_url: element
            .attr("data-orig-file")
            .filter(|url| !url.is_empty())
            .map(str::to_owned),
        width,
        height,
        alt: element
            .attr("alt")
            .map(str::trim)
            .filter(|alt| !alt.is_empty())
            .map(str::to_owned),
        caption,
        description: element.attr("data-image-description").and_then(html_text),
        link,
    })
}

/// The caption of the figure an image is in. A gallery's caption is for the whole gallery, so it
/// doesn't count.
fn figure_caption(img: ElementRef) -> Option<String> {
    let figure = img
        .ancestors()
        .filter_map(ElementRef::wrap)
        .find(|ancestor| {
            let element = ancestor.value();
            element.name() == "figure" || element.classes().any(|class| class == "wp-caption")
        })?;
    if GALLERIES.matches(&figure) {
        return None;
    }
    let caption = figure
        .children()
        .filter_map(ElementRef::wrap)
        .find(|child| {
            let element = child.value();
            element.name() == "figcaption"
                || element.classes().any(|class| class == "wp-caption-text")
        })?;
    text(caption)
}

/// Galleries inside galleries count as part of the outer one.
fn galleries(root: ElementRef) -> Vec<ReaderPostGallery> {
    root.select(&GALLERIES)
        .filter(|gallery| {
            !gallery
                .ancestors()
                .filter_map(ElementRef::wrap)
                .any(|ancestor| GALLERIES.matches(&ancestor))
        })
        .map(|gallery| ReaderPostGallery {
            images: gallery.select(&IMAGES).filter_map(image).collect(),
        })
        .filter(|gallery| !gallery.images.is_empty())
        .collect()
}

fn mentions(root: ElementRef) -> Vec<ReaderPostMention> {
    let mut mentions: Vec<ReaderPostMention> = Vec::new();
    for link in root.select(&MENTIONS) {
        let element = link.value();
        let username = element
            .attr("data-username")
            .map(str::to_owned)
            .or_else(|| Some(text(link)?.trim_start_matches('@').to_owned()))
            .filter(|username| !username.is_empty());
        let (Some(username), Some(url)) = (username, element.attr("href")) else {
            continue;
        };
        if !mentions.iter().any(|mention| mention.username == username) {
            mentions.push(ReaderPostMention {
                username,
                url: url.to_owned(),
            });
        }
    }
    mentions
}

/// An element's text, with its whitespace collapsed, or `None` if it has none.
fn text(element: ElementRef) -> Option<String> {
    let text = element.text().collect::<String>();
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!text.is_empty()).then_some(text)
}

/// Removes everything that lets the web view pick a different URL for an image than its `src`: the
/// `srcset` and `sizes` on each `<img>`, and the `<source>`s in each `<picture>`.
fn show_only_image_urls(document: &mut Html) {
    let mut sources = Vec::new();
    let mut images = Vec::new();
    for node in document.root_element().descendants() {
        let Some(element) = ElementRef::wrap(node) else {
            continue;
        };
        match element.value().name() {
            "img" => images.push(node.id()),
            "source"
                if node
                    .parent()
                    .and_then(ElementRef::wrap)
                    .is_some_and(|parent| parent.value().name() == "picture") =>
            {
                sources.push(node.id())
            }
            _ => {}
        }
    }

    for id in images {
        if let Some(mut node) = document.tree.get_mut(id)
            && let Node::Element(element) = node.value()
        {
            element
                .attrs
                .retain(|(name, _)| !matches!(&*name.local, "srcset" | "sizes"));
        }
    }
    for id in sources {
        if let Some(mut node) = document.tree.get_mut(id) {
            node.detach();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::ReaderImageSize;
    use rstest::rstest;

    #[test]
    fn finds_images() {
        let content = ReaderPostContent::new(
            r#"<p><img src="a.jpg" alt=" A photo "></p>
            <figure class="wp-block-image"><a href="https://example.com/a">
              <img src="https://example.files.wordpress.com/b.jpg?w=1024"
                srcset="https://example.files.wordpress.com/b.jpg?w=300 300w, https://example.files.wordpress.com/b.jpg?w=1024 1024w"
                data-orig-file="https://example.files.wordpress.com/b.jpg" data-orig-size="4032,3024" width="1024" height="768">
            </a><figcaption>A <em>caption</em></figcaption></figure>
            <p>Hi <img class="emoji" src="smile.svg"> <img class="wp-smiley" src="wink.gif"></p>"#,
            None,
        );
        assert_eq!(
            content.images(),
            [
                ReaderPostImage {
                    url: "a.jpg".to_owned(),
                    sizes: vec![],
                    original_url: None,
                    width: None,
                    height: None,
                    alt: Some("A photo".to_owned()),
                    caption: None,
                    description: None,
                    link: None,
                },
                ReaderPostImage {
                    url: "https://example.files.wordpress.com/b.jpg?w=1024".to_owned(),
                    sizes: vec![
                        ReaderImageSize {
                            url: "https://example.files.wordpress.com/b.jpg?w=300".to_owned(),
                            width: 300,
                        },
                        ReaderImageSize {
                            url: "https://example.files.wordpress.com/b.jpg?w=1024".to_owned(),
                            width: 1024,
                        },
                    ],
                    original_url: Some("https://example.files.wordpress.com/b.jpg".to_owned()),
                    width: Some(4032),
                    height: Some(3024),
                    alt: None,
                    caption: Some("A caption".to_owned()),
                    description: None,
                    link: Some("https://example.com/a".to_owned()),
                },
            ]
        );
    }

    #[rstest]
    #[case::attributes(
        r#"<img src="a.jpg" width="600" height="400px">"#,
        Some(600),
        Some(400)
    )]
    #[case::original_size_wins(
        r#"<img src="a.jpg" width="600" height="400" data-orig-size="3000,2000">"#,
        Some(3000),
        Some(2000)
    )]
    #[case::from_the_url(
        r#"<img src="https://example.files.wordpress.com/a.jpg?w=1024&amp;h=768">"#,
        Some(1024),
        Some(768)
    )]
    #[case::attributes_win_over_the_url(
        r#"<img src="https://example.files.wordpress.com/a.jpg?w=1024" width="300">"#,
        Some(300),
        None
    )]
    #[case::unknown(r#"<img src="a.jpg" width="auto">"#, None, None)]
    fn finds_image_sizes(
        #[case] html: &str,
        #[case] width: Option<u32>,
        #[case] height: Option<u32>,
    ) {
        let image = &ReaderPostContent::new(html, None).images()[0];
        assert_eq!((image.width, image.height), (width, height));
    }

    #[rstest]
    #[case::wordpress_com(
        r#"<img src="a.jpg" data-image-caption="&lt;p&gt;A  caption&lt;/p&gt;">"#,
        Some("A caption")
    )]
    #[case::classic_editor(
        r#"<div class="wp-caption"><img src="a.jpg"><p class="wp-caption-text">A caption</p></div>"#,
        Some("A caption")
    )]
    #[case::gallery_caption_isnt_the_image_caption(
        r#"<figure class="wp-block-gallery"><img src="a.jpg"><figcaption>The gallery</figcaption></figure>"#,
        None
    )]
    #[case::image_in_a_gallery(
        r#"<figure class="wp-block-gallery"><figure class="wp-block-image"><img src="a.jpg"><figcaption>One</figcaption></figure><figcaption>The gallery</figcaption></figure>"#,
        Some("One")
    )]
    fn finds_image_captions(#[case] html: &str, #[case] caption: Option<&str>) {
        let image = &ReaderPostContent::new(html, None).images()[0];
        assert_eq!(image.caption.as_deref(), caption);
    }

    #[test]
    fn finds_galleries() {
        let content = ReaderPostContent::new(
            r#"<img src="before.jpg">
            <figure class="wp-block-gallery has-nested-images">
              <figure class="wp-block-image"><img src="a.jpg"></figure>
              <figure class="wp-block-image"><img src="b.jpg"></figure>
            </figure>
            <div class="tiled-gallery"><div class="gallery"><img src="c.jpg"></div></div>
            <ul class="wp-block-gallery"><li class="blocks-gallery-item"><figure><img src="d.jpg"></figure></li></ul>
            <figure class="wp-block-gallery"></figure>"#,
            None,
        );
        let galleries: Vec<Vec<String>> = content
            .galleries()
            .into_iter()
            .map(|gallery| gallery.images.into_iter().map(|image| image.url).collect())
            .collect();
        assert_eq!(
            galleries,
            [vec!["a.jpg", "b.jpg"], vec!["c.jpg"], vec!["d.jpg"]]
        );
        assert_eq!(content.images().len(), 5);
    }

    #[test]
    fn finds_mentions() {
        let content = ReaderPostContent::new(
            r#"<p><a href="https://site.wordpress.com/mentions/tester/" class="__p2-hovercard mention" data-type="fragment-mention" data-username="tester"><span class="mentions-prefix">@</span>tester</a>
            and <a class="mention" href="https://site.wordpress.com/mentions/other/">@other</a>,
            <a class="mention" href="https://site.wordpress.com/mentions/tester/" data-username="tester">@tester</a> again,
            and <a href="https://www.instagram.com/someone/">@someone</a> on Instagram.</p>"#,
            None,
        );
        assert_eq!(
            content.mentions(),
            [
                ReaderPostMention {
                    username: "tester".to_owned(),
                    url: "https://site.wordpress.com/mentions/tester/".to_owned(),
                },
                ReaderPostMention {
                    username: "other".to_owned(),
                    url: "https://site.wordpress.com/mentions/other/".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn offline_html_only_uses_image_urls() {
        let content = ReaderPostContent::new(
            r#"<img src="a.jpg" srcset="a-300.jpg 300w" sizes="100vw"><picture><source srcset="b.webp" type="image/webp"><img src="b.jpg"></picture><video><source src="c.mp4"></video>"#,
            None,
        );
        assert_eq!(
            content.offline_html(),
            r#"<img src="a.jpg"><picture><img src="b.jpg"></picture><video controls=""><source src="c.mp4"></video>"#
        );
        assert!(content.html().contains(r#"srcset="a-300.jpg 300w""#));
    }

    #[test]
    fn lists_every_image_to_download_for_offline() {
        let content = ReaderPostContent::new(
            r#"<img src="a.jpg" srcset="a-300.jpg 300w"><img class="emoji" src="smile.svg"><img src="a.jpg"><img src="">"#,
            None,
        );
        assert_eq!(content.offline_image_urls(), ["a.jpg", "smile.svg"]);
    }

    #[test]
    fn adds_photon_sizes_to_images_with_one_size() {
        let content = ReaderPostContent::new(
            r#"<script src="https://platform.twitter.com/widgets.js"></script>
            <img src="https://example.com/a.jpg" width="1000" height="500">
            <img src="https://example.com/b.jpg" srcset="https://example.com/b-300.jpg 300w">
            <img src="https://example.com/c.svg">"#,
            None,
        );
        let resized = content.with_photon_sizes();

        let images = resized.images();
        let widths: Vec<u32> = images[0].sizes.iter().map(|size| size.width).collect();
        assert_eq!(widths, [400, 800, 1000]);
        assert_eq!(
            images[0].url_for_size(700),
            "https://i0.wp.com/example.com/a.jpg?quality=80&w=800&ssl=1"
        );
        assert_eq!(images[1].sizes, content.images()[1].sizes);
        assert_eq!(images[2].sizes, []);
        assert_eq!(resized.scripts(), content.scripts());
        // The offline post only uses each image's `src`, so it doesn't change.
        assert_eq!(resized.offline_html(), content.offline_html());
    }

    #[test]
    fn leaves_a_post_without_photon_sizes_alone() {
        let content =
            ReaderPostContent::new(r#"<p>Text <img src="https://example.com/a.svg"></p>"#, None);
        assert_eq!(content.with_photon_sizes().html(), content.html());
    }

    #[rstest]
    #[case::src("https://example.files.wordpress.com/b.jpg?w=1024", true)]
    #[case::original_without_a_query("https://example.files.wordpress.com/b.jpg", true)]
    #[case::photon(
        "https://i0.wp.com/example.files.wordpress.com/b.jpg?resize=600%2C400",
        true
    )]
    #[case::another_image("https://example.files.wordpress.com/c.jpg", false)]
    #[case::not_a_url("featured image", false)]
    fn knows_which_images_it_includes(#[case] url: &str, #[case] included: bool) {
        let content = ReaderPostContent::new(
            r#"<img src="https://example.files.wordpress.com/b.jpg?w=1024" srcset="https://example.files.wordpress.com/b.jpg?w=300 300w">"#,
            None,
        );
        assert_eq!(content.includes_image(url), included);
    }

    #[rstest]
    #[case::src("https://example.com/b-1024x768.jpg", Some((1, 1)))]
    #[case::linked_original("https://example.com/b.jpg", Some((1, 1)))]
    #[case::photon_copy("https://i0.wp.com/example.com/c.jpg?w=800", Some((1, 2)))]
    #[case::first_gallery("https://example.com/a.jpg", Some((0, 0)))]
    #[case::outside_a_gallery("https://example.com/single.jpg", None)]
    #[case::not_in_the_post("https://example.com/other.jpg", None)]
    fn finds_where_an_image_is_in_a_gallery(
        #[case] url: &str,
        #[case] expected: Option<(usize, u32)>,
    ) {
        let content = ReaderPostContent::new(
            r#"<img src="https://example.com/single.jpg">
            <figure class="wp-block-gallery"><figure class="wp-block-image"><img src="https://example.com/a.jpg"></figure></figure>
            <figure class="wp-block-gallery">
              <figure class="wp-block-image"><img src="https://example.com/z.jpg"></figure>
              <figure class="wp-block-image"><a href="https://example.com/b.jpg"><img src="https://example.com/b-1024x768.jpg"></a></figure>
              <figure class="wp-block-image"><img src="https://example.com/c.jpg"></figure>
            </figure>"#,
            None,
        );
        let position = content.gallery_position(url);
        assert_eq!(
            position.as_ref().map(|position| {
                let gallery = content
                    .galleries()
                    .iter()
                    .position(|gallery| *gallery == position.gallery)
                    .unwrap();
                (gallery, position.index)
            }),
            expected
        );
    }

    #[test]
    fn lists_what_to_preload() {
        let content = ReaderPostContent::new(
            r#"<p><img src="https://example.com/a.jpg" width="1024" srcset="https://example.com/a-300.jpg 300w, https://example.com/a-768.jpg 768w, https://example.com/a-1536.jpg 1536w, https://example.com/a.jpg 1024w"></p>
            <p><img src="https://example.com/small.jpg" width="150" srcset="https://example.com/small-150.jpg 150w, https://example.com/small-300.jpg 300w"></p>
            <p><img src="https://example.com/huge.jpg" srcset="https://example.com/huge-300.jpg 300w"></p>
            <p><img src="https://example.com/plain.png"><img src="https://example.com/plain.png"></p>
            <video poster="https://example.com/poster.jpg" src="https://example.com/v.mp4"></video>
            <img src="data:image/gif;base64,R0lGODlhAQABAAAAACw=">
            <blockquote class="twitter-tweet"></blockquote>"#,
            None,
        );
        assert_eq!(
            content.preload_urls(390, 2.0),
            [
                // 390 points at 2x is 780 pixels.
                "https://example.com/a.jpg",
                // Narrower than the screen: 150 points at 2x.
                "https://example.com/small-300.jpg",
                // No size is wide enough, so the widest.
                "https://example.com/huge-300.jpg",
                "https://example.com/plain.png",
                "https://example.com/poster.jpg",
                "https://platform.twitter.com/widgets.js",
            ]
        );
    }
}
