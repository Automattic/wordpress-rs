use url::{Position, Url};

/// An image in a Reader post.
#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Record)]
pub struct ReaderPostImage {
    /// The image the post shows, from its `src`. Reading offline, it's the only one the post shows.
    pub url: String,
    /// The other sizes the post offers, from its `srcset`.
    pub sizes: Vec<ReaderImageSize>,
    /// The full-size original, on sites that say where it is (WordPress.com's `data-orig-file`).
    pub original_url: Option<String>,
    /// The image's width in pixels: the original's, if the site says (`data-orig-size`), or else
    /// the `width` attribute, or else the size the URL asks for (`?w=1024`, or `?resize=1024,768`).
    pub width: Option<u32>,
    /// The image's height in pixels, from the same place as [`width`](Self::width).
    pub height: Option<u32>,
    /// The image's text alternative.
    pub alt: Option<String>,
    /// The image's caption, as plain text.
    pub caption: Option<String>,
    /// The image's description, as plain text, on sites that have one (WordPress.com's
    /// `data-image-description`).
    pub description: Option<String>,
    /// Where the image links to, if it's a link.
    pub link: Option<String>,
}

/// One size of an image, from its `srcset`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Record)]
pub struct ReaderImageSize {
    pub url: String,
    /// The width in pixels.
    pub width: u32,
}

#[uniffi::export]
impl ReaderPostImage {
    /// The URL to show the image at `max_dimension` pixels on its longer side, like in a
    /// full-screen image viewer.
    ///
    /// That's the smallest size that fills it. If none does, it's the original, then the largest
    /// size, then [`url`](Self::url).
    pub fn url_for_size(&self, max_dimension: u32) -> String {
        // A portrait image's longer side is its height, so it needs to be wider than
        // `max_dimension` to fill it.
        let required_width = match (self.width, self.height) {
            (Some(width), Some(height)) if height > width => {
                (u64::from(max_dimension) * u64::from(width)).div_ceil(u64::from(height)) as u32
            }
            _ => max_dimension,
        };

        let mut sizes: Vec<_> = self.sizes.iter().collect();
        sizes.sort_by_key(|size| size.width);
        sizes
            .iter()
            .find(|size| size.width >= required_width)
            .map(|size| &size.url)
            .or(self.original_url.as_ref())
            .or(sizes.last().map(|size| &size.url))
            .unwrap_or(&self.url)
            .clone()
    }
}

impl ReaderPostImage {
    /// Every URL the post has for this image.
    pub(super) fn urls(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.url.as_str())
            .chain(self.original_url.as_deref())
            .chain(self.sizes.iter().map(|size| size.url.as_str()))
    }
}

/// The sizes in a `srcset` that give a width (`300w`), in order.
///
/// URLs can contain commas, like Photon's `?resize=300,200`, so this follows the HTML spec's parsing
/// rather than splitting on them.
pub(super) fn parse_srcset(srcset: &str) -> Vec<ReaderImageSize> {
    srcset_candidates(srcset)
        .into_iter()
        .filter_map(|(url, descriptors)| {
            let width = descriptors
                .split_ascii_whitespace()
                .find_map(|descriptor| descriptor.strip_suffix('w')?.parse().ok())?;
            Some(ReaderImageSize {
                url: url.to_owned(),
                width,
            })
        })
        .collect()
}

/// Rewrites the URLs in a `srcset`, keeping each one's descriptors. `rewrite` returns `None` to
/// leave a URL as it is.
pub(super) fn rewrite_srcset(srcset: &str, rewrite: impl Fn(&str) -> Option<String>) -> String {
    srcset_candidates(srcset)
        .into_iter()
        .map(|(url, descriptors)| {
            let url = rewrite(url).unwrap_or_else(|| url.to_owned());
            match descriptors.trim() {
                "" => url,
                descriptors => format!("{url} {descriptors}"),
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Each URL in a `srcset`, with its descriptors, like `300w`.
pub(super) fn srcset_candidates(srcset: &str) -> Vec<(&str, &str)> {
    let mut candidates = Vec::new();
    let mut rest = srcset;
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == ',');
        if rest.is_empty() {
            return candidates;
        }

        let url_end = rest
            .find(|c: char| c.is_ascii_whitespace())
            .unwrap_or(rest.len());
        let (url, after_url) = rest.split_at(url_end);
        // A URL that ends in a comma has no descriptors.
        let (url, descriptors, next) = match url.strip_suffix(',') {
            Some(url) => (url.trim_end_matches(','), "", after_url),
            None => {
                let end = after_url.find(',').unwrap_or(after_url.len());
                (url, &after_url[..end], &after_url[end..])
            }
        };
        rest = next;
        candidates.push((url, descriptors));
    }
}

/// Photon, WordPress.com's image CDN, serves from `i0.wp.com`, `i1.wp.com` and so on.
fn is_photon_host(host: &str) -> bool {
    host.strip_suffix(".wp.com").is_some_and(|server| {
        server.len() >= 2
            && server.starts_with('i')
            && server[1..].chars().all(|c| c.is_ascii_digit())
    })
}

/// The widths [`photon_sizes`] offers.
const PHOTON_WIDTHS: &[u32] = &[400, 800, 1200, 1600, 2000];

/// Smaller sizes of an image, made by Photon, WordPress.com's image CDN.
///
/// These are the URLs the apps' Photon helpers make: `https://i0.wp.com/<host><path>`, with
/// `quality=80`, the width, and `ssl=1` if the original is HTTPS. Photon won't make an image bigger
/// than it is, so the sizes stop at `width` when it's known.
///
/// Photon resizes JPEG, PNG, GIF, WebP, HEIC and AVIF images. It sends WebP, HEIC and AVIF back as
/// JPEG, which helps on Android, whose web view can't show HEIC.
pub(super) fn photon_sizes(src: &str, width: Option<u32>) -> Vec<ReaderImageSize> {
    let Ok(url) = Url::parse(src) else {
        return Vec::new();
    };
    let extension = url
        .path()
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase());
    let resizable = matches!(url.scheme(), "http" | "https")
        && matches!(
            extension.as_deref(),
            Some("jpg" | "jpeg" | "png" | "gif" | "webp" | "heic" | "avif")
        )
        // WordPress.com's site screenshots resize themselves.
        && !url.path().contains("/mshots/");
    let Some(host) = url.host_str().filter(|_| resizable) else {
        return Vec::new();
    };

    let (base, ssl) = if is_photon_host(host) {
        let ssl = url
            .query_pairs()
            .any(|(key, value)| key == "ssl" && value == "1");
        (url[..Position::AfterPath].to_owned(), ssl)
    } else {
        let address = &url[Position::BeforeHost..Position::AfterPath];
        (
            format!("https://i0.wp.com/{address}"),
            url.scheme() == "https",
        )
    };
    let ssl = if ssl { "&ssl=1" } else { "" };

    let mut widths: Vec<u32> = PHOTON_WIDTHS
        .iter()
        .copied()
        .filter(|size| width.is_none_or(|width| *size < width))
        .collect();
    widths.extend(width.filter(|width| *width > 0));
    widths
        .into_iter()
        .map(|width| ReaderImageSize {
            url: format!("{base}?quality=80&w={width}{ssl}"),
            width,
        })
        .collect()
}

/// Parses WordPress.com's `data-orig-size`, like `4032,3024`.
pub(super) fn parse_size(size: &str) -> Option<(u32, u32)> {
    let (width, height) = size.split_once(',')?;
    Some((width.trim().parse().ok()?, height.trim().parse().ok()?))
}

/// The size an image URL asks for, from WordPress.com's `?w=1024&h=768` or Photon's
/// `?resize=1024,768`.
pub(super) fn size_from_query(url: &str) -> (Option<u32>, Option<u32>) {
    let Ok(url) = Url::parse(url) else {
        return (None, None);
    };
    let (mut width, mut height) = (None, None);
    for (key, value) in url.query_pairs() {
        match &*key {
            "w" => width = value.parse().ok(),
            "h" => height = value.parse().ok(),
            "resize" | "fit" => {
                if let Some((w, h)) = parse_size(&value) {
                    (width, height) = (Some(w), Some(h));
                }
            }
            _ => {}
        }
    }
    (width, height)
}

/// Parses a `width` or `height` attribute, which can be `600` or `600px`.
pub(super) fn parse_dimension(dimension: &str) -> Option<u32> {
    dimension.trim().trim_end_matches("px").parse().ok()
}

/// What two URLs for the same image have in common: the host and path, without the Photon CDN in
/// front, and without the size WordPress adds to the names of smaller copies (`-1024x768`, or
/// `-scaled` for a large original).
pub(super) fn image_key(url: &str) -> Option<String> {
    let url = match url.strip_prefix("//") {
        Some(rest) => Url::parse(&format!("https://{rest}")),
        None => Url::parse(url),
    }
    .ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    let path = url.path();

    // Photon serves `https://i0.wp.com/example.com/image.jpg` from `https://example.com/image.jpg`.
    let key = if is_photon_host(&host) {
        let path = path.trim_start_matches('/');
        let (origin, path) = path.split_once('/').unwrap_or((path, ""));
        format!("{}/{path}", origin.to_ascii_lowercase())
    } else {
        format!("{host}{path}")
    };
    Some(without_size_suffix(&key))
}

/// A WordPress image file name without its size suffix: `photo-1024x768.jpg` and `photo-scaled.jpg`
/// are both copies of `photo.jpg`.
fn without_size_suffix(path: &str) -> String {
    let Some((stem, extension)) = path.rsplit_once('.') else {
        return path.to_owned();
    };
    if extension.contains('/') {
        return path.to_owned();
    }
    let stem = stem.strip_suffix("-scaled").unwrap_or(stem);
    let stem = stem
        .rsplit_once('-')
        .filter(|(_, size)| {
            size.split_once('x').is_some_and(|(width, height)| {
                !width.is_empty()
                    && !height.is_empty()
                    && width
                        .chars()
                        .chain(height.chars())
                        .all(|c| c.is_ascii_digit())
            })
        })
        .map_or(stem, |(stem, _)| stem);
    format!("{stem}.{extension}")
}

/// The images a page of `root` loads in a web view `width` points wide, with `scale` pixels to a
/// point, in order: each image at the size its `srcset` offers for that screen, and videos'
/// posters.
///
/// The web view picks the size: this picks the smallest one at least as wide as the image shows,
/// which is what WebKit and Chromium do. An image shows at the screen's width, or its own `width`
/// if that's narrower. Images in gallery columns show narrower than that, so they get a larger size
/// than the web view picks.
pub(super) fn image_urls_to_preload(
    root: scraper::ElementRef,
    width: u32,
    scale: f64,
) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    for element in root.descendants().filter_map(scraper::ElementRef::wrap) {
        let element = element.value();
        let url = match element.name() {
            "img" => {
                let shown_width = element
                    .attr("width")
                    .and_then(parse_dimension)
                    .filter(|shown| *shown > 0)
                    .map_or(width, |shown| shown.min(width));
                let needed = (f64::from(shown_width) * scale.max(1.0)).ceil() as u32;
                let mut sizes = element.attr("srcset").map(parse_srcset).unwrap_or_default();
                sizes.sort_by_key(|size| size.width);
                let index = sizes
                    .iter()
                    .position(|size| size.width >= needed)
                    .or(sizes.len().checked_sub(1));
                match index {
                    Some(index) => Some(sizes.swap_remove(index).url),
                    None => element.attr("src").map(str::to_owned),
                }
            }
            "video" => element.attr("poster").map(str::to_owned),
            _ => None,
        };
        let Some(url) = url else {
            continue;
        };
        let url = url.trim();
        // Some Jetpack sites send just their address as an image's `src`, which is a page.
        let is_image_on_the_web = Url::parse(url)
            .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.path() != "/");
        if is_image_on_the_web && !urls.iter().any(|existing| existing == url) {
            urls.push(url.to_owned());
        }
    }
    urls
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn size(url: &str, width: u32) -> ReaderImageSize {
        ReaderImageSize {
            url: url.to_owned(),
            width,
        }
    }

    #[rstest]
    #[case::wordpress("a.jpg 300w, b.jpg 1024w", vec![size("a.jpg", 300), size("b.jpg", 1024)])]
    #[case::commas_in_urls(
        "https://i0.wp.com/a.jpg?resize=300,200 300w, https://i0.wp.com/a.jpg?resize=600,400 600w",
        vec![size("https://i0.wp.com/a.jpg?resize=300,200", 300), size("https://i0.wp.com/a.jpg?resize=600,400", 600)]
    )]
    #[case::density_descriptors("a.jpg 1x, b.jpg 2x", vec![])]
    #[case::no_descriptors("a.jpg, b.jpg 600w", vec![size("b.jpg", 600)])]
    #[case::extra_whitespace("  a.jpg   300w ,\n b.jpg 600w  ", vec![size("a.jpg", 300), size("b.jpg", 600)])]
    #[case::empty("", vec![])]
    fn parses_srcset(#[case] srcset: &str, #[case] expected: Vec<ReaderImageSize>) {
        assert_eq!(parse_srcset(srcset), expected);
    }

    #[test]
    fn rewrites_srcset_urls() {
        let rewritten =
            rewrite_srcset("/a.jpg 300w, https://example.com/b.jpg 2x, /c.jpg", |url| {
                url.starts_with('/')
                    .then(|| format!("https://example.com{url}"))
            });
        assert_eq!(
            rewritten,
            "https://example.com/a.jpg 300w, https://example.com/b.jpg 2x, https://example.com/c.jpg"
        );
    }

    fn image(
        width: Option<u32>,
        height: Option<u32>,
        original_url: Option<&str>,
    ) -> ReaderPostImage {
        ReaderPostImage {
            url: "src.jpg".to_owned(),
            sizes: vec![
                size("300.jpg", 300),
                size("1024.jpg", 1024),
                size("600.jpg", 600),
            ],
            original_url: original_url.map(str::to_owned),
            width,
            height,
            alt: None,
            caption: None,
            description: None,
            link: None,
        }
    }

    #[rstest]
    #[case::smallest_that_fills(image(None, None, None), 500, "600.jpg")]
    #[case::exact(image(None, None, None), 600, "600.jpg")]
    #[case::portrait_needs_less_width(image(Some(3000), Some(4000), None), 800, "600.jpg")]
    #[case::landscape(image(Some(4000), Some(3000), None), 800, "1024.jpg")]
    #[case::original_when_nothing_fills(
        image(None, None, Some("original.jpg")),
        2000,
        "original.jpg"
    )]
    #[case::largest_without_an_original(image(None, None, None), 2000, "1024.jpg")]
    fn picks_a_url_for_a_size(
        #[case] image: ReaderPostImage,
        #[case] max_dimension: u32,
        #[case] expected: &str,
    ) {
        assert_eq!(image.url_for_size(max_dimension), expected);
    }

    #[test]
    fn falls_back_to_the_src() {
        let image = ReaderPostImage {
            sizes: vec![],
            ..image(None, None, None)
        };
        assert_eq!(image.url_for_size(600), "src.jpg");
    }

    #[rstest]
    #[case::wordpress_com("https://example.files.wordpress.com/a.jpg?w=1024&h=768", (Some(1024), Some(768)))]
    #[case::width_only("https://example.files.wordpress.com/a.jpg?w=1024", (Some(1024), None))]
    #[case::photon("https://i0.wp.com/example.com/a.jpg?resize=1024%2C768&ssl=1", (Some(1024), Some(768)))]
    #[case::none("https://example.com/a.jpg", (None, None))]
    #[case::not_a_url("a.jpg?w=1024", (None, None))]
    fn finds_the_size_a_url_asks_for(#[case] url: &str, #[case] size: (Option<u32>, Option<u32>)) {
        assert_eq!(size_from_query(url), size);
    }

    #[test]
    fn makes_photon_sizes() {
        let sizes = photon_sizes("https://example.com/wp-content/a.jpg?ver=2", Some(1000));
        assert_eq!(
            sizes,
            [
                size(
                    "https://i0.wp.com/example.com/wp-content/a.jpg?quality=80&w=400&ssl=1",
                    400
                ),
                size(
                    "https://i0.wp.com/example.com/wp-content/a.jpg?quality=80&w=800&ssl=1",
                    800
                ),
                size(
                    "https://i0.wp.com/example.com/wp-content/a.jpg?quality=80&w=1000&ssl=1",
                    1000
                ),
            ]
        );
    }

    #[rstest]
    #[case::http(
        "http://example.com/a.PNG",
        "https://i0.wp.com/example.com/a.PNG?quality=80&w=400"
    )]
    #[case::port(
        "https://example.com:8080/a.gif",
        "https://i0.wp.com/example.com:8080/a.gif?quality=80&w=400&ssl=1"
    )]
    #[case::already_photon(
        "https://i2.wp.com/example.com/a.jpg?resize=300%2C200&ssl=1",
        "https://i2.wp.com/example.com/a.jpg?quality=80&w=400&ssl=1"
    )]
    #[case::unknown_width_gets_every_size(
        "https://example.com/a.jpeg",
        "https://i0.wp.com/example.com/a.jpeg?quality=80&w=400&ssl=1"
    )]
    #[case::heic(
        "https://example.com/a.heic",
        "https://i0.wp.com/example.com/a.heic?quality=80&w=400&ssl=1"
    )]
    fn makes_photon_urls(#[case] src: &str, #[case] first: &str) {
        assert_eq!(photon_sizes(src, None)[0].url, first);
    }

    #[rstest]
    #[case::svg("https://example.com/a.svg")]
    #[case::no_extension("https://example.com/image")]
    #[case::screenshot("https://s0.wp.com/mshots/v1/https%3A%2F%2Fexample.com.jpg")]
    #[case::relative("/wp-content/a.jpg")]
    #[case::data("data:image/png;base64,AAAA")]
    fn doesnt_make_photon_sizes(#[case] src: &str) {
        assert_eq!(photon_sizes(src, None), []);
    }

    #[test]
    fn makes_every_photon_size_for_an_unknown_width() {
        assert_eq!(
            photon_sizes("https://example.com/a.jpg", None).len(),
            PHOTON_WIDTHS.len()
        );
    }

    #[rstest]
    #[case::same("https://example.com/a.jpg", "https://example.com/a.jpg")]
    #[case::query(
        "https://example.files.wordpress.com/a.jpg?w=1024",
        "https://example.files.wordpress.com/a.jpg"
    )]
    #[case::scheme("http://example.com/a.jpg", "https://example.com/a.jpg")]
    #[case::protocol_relative("//example.com/a.jpg", "https://example.com/a.jpg")]
    #[case::host_case("https://EXAMPLE.com/a.jpg", "https://example.com/a.jpg")]
    #[case::photon(
        "https://i2.wp.com/example.com/a.jpg?resize=300%2C200",
        "https://example.com/a.jpg"
    )]
    fn matches_urls_for_the_same_image(#[case] a: &str, #[case] b: &str) {
        assert_eq!(image_key(a), image_key(b));
    }

    #[rstest]
    #[case::wordpress_size(
        "https://example.com/uploads/a-1024x768.jpg",
        "https://example.com/uploads/a.jpg"
    )]
    #[case::scaled(
        "https://example.com/uploads/a-scaled.jpg",
        "https://example.com/uploads/a.jpg"
    )]
    #[case::photon_and_size(
        "https://i2.wp.com/example.com/uploads/a-1024x779.jpg?ssl=1",
        "http://example.com/uploads/a.jpg"
    )]
    fn matches_sizes_of_the_same_image(#[case] a: &str, #[case] b: &str) {
        assert_eq!(image_key(a), image_key(b));
    }

    #[rstest]
    #[case::not_a_size(
        "https://example.com/uploads/a-final.jpg",
        "https://example.com/uploads/a.jpg"
    )]
    #[case::partial_size(
        "https://example.com/uploads/a-1024x.jpg",
        "https://example.com/uploads/a.jpg"
    )]
    #[case::path("https://example.com/a.jpg", "https://example.com/b.jpg")]
    #[case::host("https://example.com/a.jpg", "https://example.org/a.jpg")]
    #[case::not_photon("https://i0.wp.com.example.com/a.jpg", "https://example.com/a.jpg")]
    fn tells_different_images_apart(#[case] a: &str, #[case] b: &str) {
        assert_ne!(image_key(a), image_key(b));
    }
}
