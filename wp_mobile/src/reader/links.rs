use url::Url;

use super::anchors::percent_decode;
use super::page::reader_page_base_url;

/// What a link in a Reader post goes to, so the app can open it the right way.
#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum ReaderLink {
    /// Somewhere in the post itself, like a footnote. `ReaderPostContent.anchors` says what's
    /// there. The page's script reports taps on these links itself, so the app mostly sees them in
    /// links from elsewhere to the post.
    Anchor { id: String },
    /// A comment on the post.
    Comment { comment_id: u64 },
    /// A tag's posts in the Reader.
    Tag { slug: String },
    /// A site's posts in the Reader.
    Site { site_id: u64 },
    /// A feed's posts in the Reader.
    Feed { feed_id: u64 },
    /// A post the Reader can show, by its site's ID.
    SitePost { site_id: u64, post_id: u64 },
    /// A post the Reader can show, by its feed's ID.
    FeedPost { feed_id: u64, post_id: u64 },
    /// A WordPress.com post, by its address, which the Reader can look up.
    WordPressComPost { url: String },
    /// Someone a P2 post mentions.
    Mention { username: String },
    /// An image, for the app's image viewer. If it's in one of the post's galleries,
    /// `ReaderPostContent.galleryPosition` finds which.
    Image { url: String },
    /// A video, for the platform's player: a video file or a YouTube video.
    Video { url: String },
    /// A file to download, like a PDF, a spreadsheet, an archive or a sound file.
    File { url: String },
    /// A web story, which needs the whole screen.
    Story { url: String },
    /// A phone number to call.
    Phone { number: String },
    /// A phone number to send a text message to.
    TextMessage { number: String },
    /// An email address to write to.
    Email { address: String },
    /// Any other web page, or a link the app can only hand to the system.
    External { url: String },
}

/// What a link in a Reader post goes to. `url` is the link's address, as the web view reports it.
/// `post_url` is the post's address, so links to somewhere in the post are recognised.
///
/// Images, videos and files are recognised by their file extensions: the ones WordPress allows
/// uploading. A link to an image in one of the post's galleries is still an image; use
/// `ReaderPostContent.galleryPosition` to open the gallery at it.
#[uniffi::export(default(post_url = None))]
pub fn reader_link(url: &str, post_url: Option<String>) -> ReaderLink {
    let post_url = post_url.as_deref().and_then(|url| Url::parse(url).ok());
    let base = post_url
        .clone()
        .or_else(|| Url::parse(&reader_page_base_url()).ok());
    let Some(link) = Url::options()
        .base_url(base.as_ref())
        .parse(url.trim())
        .ok()
    else {
        return ReaderLink::External {
            url: url.to_owned(),
        };
    };

    match link.scheme() {
        "http" | "https" => {}
        "mailto" => {
            return ReaderLink::Email {
                address: percent_decode(link.path()),
            };
        }
        "tel" => {
            return ReaderLink::Phone {
                number: percent_decode(link.path()),
            };
        }
        "sms" => {
            return ReaderLink::TextMessage {
                number: percent_decode(link.path()),
            };
        }
        // Discover posts link to the sites they feature this way.
        "wordpress" if link.host_str() == Some("blogpreview") => {
            if let Some(site_id) = query_number(&link, "blogId") {
                return ReaderLink::Site { site_id };
            }
            return external(&link);
        }
        _ => return external(&link),
    }

    if let Some(fragment) = link.fragment().filter(|fragment| !fragment.is_empty()) {
        let reader_page = Url::parse(&reader_page_base_url()).ok();
        let in_this_post = [post_url.as_ref(), reader_page.as_ref()]
            .into_iter()
            .flatten()
            .any(|page| is_same_page(&link, page));
        if in_this_post {
            let id = percent_decode(fragment);
            return match id.strip_prefix("comment-").and_then(|id| id.parse().ok()) {
                Some(comment_id) => ReaderLink::Comment { comment_id },
                None => ReaderLink::Anchor { id },
            };
        }
    }

    let host = link.host_str().unwrap_or_default().to_ascii_lowercase();
    let segments: Vec<&str> = link
        .path_segments()
        .map(|segments| segments.filter(|segment| !segment.is_empty()).collect())
        .unwrap_or_default();

    if (host == "wordpress.com" || host == "www.wordpress.com")
        && let Some(reader_link) = reader_route(&segments)
    {
        return reader_link;
    }
    if let ["mentions", username] = segments.as_slice() {
        return ReaderLink::Mention {
            username: percent_decode(username),
        };
    }
    if link
        .query_pairs()
        .any(|(name, _)| name.contains("wp-story"))
    {
        return ReaderLink::Story { url: link.into() };
    }
    if is_youtube_video(&link, &host, &segments) {
        return ReaderLink::Video { url: link.into() };
    }

    let extension = segments
        .last()
        .and_then(|name| name.rsplit_once('.'))
        .map(|(_, extension)| extension.to_ascii_lowercase());
    match extension.as_deref() {
        Some(extension) if IMAGE_EXTENSIONS.contains(&extension) => {
            return ReaderLink::Image { url: link.into() };
        }
        Some(extension) if VIDEO_EXTENSIONS.contains(&extension) => {
            return ReaderLink::Video { url: link.into() };
        }
        Some(extension) if FILE_EXTENSIONS.contains(&extension) => {
            return ReaderLink::File { url: link.into() };
        }
        _ => {}
    }

    if is_wordpress_com_post(&host, &segments) {
        return ReaderLink::WordPressComPost { url: link.into() };
    }
    external(&link)
}

/// The image file extensions WordPress allows uploading.
const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "jpe", "gif", "png", "bmp", "tif", "tiff", "ico", "heic", "heif", "webp", "avif",
];

/// The video file extensions WordPress allows uploading.
const VIDEO_EXTENSIONS: &[&str] = &[
    "3g2", "3gp", "3gpp", "asf", "asx", "avi", "divx", "dv", "flv", "m4v", "mkv", "mov", "mp4",
    "mpeg", "mpg", "mpv", "ogm", "ogv", "qt", "rm", "vob", "webm", "wmv",
];

/// The other file extensions WordPress allows uploading, in its order: documents, spreadsheets,
/// presentations, archives and sound files. And e-books.
const FILE_EXTENSIONS: &[&str] = &[
    "doc", "docx", "docm", "dotm", "odt", "pages", "pdf", "xps", "oxps", "rtf", "wp", "wpd", "psd",
    "xcf", "numbers", "ods", "xls", "xlsx", "xlsm", "xlsb", "key", "ppt", "pptx", "pptm", "pps",
    "ppsx", "ppsm", "sldx", "sldm", "odp", "bz2", "cab", "dmg", "gz", "rar", "sea", "sit", "sqx",
    "tar", "tgz", "zip", "7z", "aac", "ac3", "aif", "aiff", "flac", "m3a", "m4a", "m4b", "mka",
    "mp1", "mp2", "mp3", "ogg", "oga", "ram", "wav", "wma", "epub",
];

/// The Reader's own pages on WordPress.com that the app shows natively.
fn reader_route(segments: &[&str]) -> Option<ReaderLink> {
    let number = |segment: &str| segment.parse::<u64>().ok();
    match segments {
        ["tag", slug] => Some(ReaderLink::Tag {
            slug: percent_decode(slug),
        }),
        ["read" | "reader", "blogs", site_id] => Some(ReaderLink::Site {
            site_id: number(site_id)?,
        }),
        ["read" | "reader", "feeds", feed_id] => Some(ReaderLink::Feed {
            feed_id: number(feed_id)?,
        }),
        ["read" | "reader", "blogs", site_id, "posts", post_id] => Some(ReaderLink::SitePost {
            site_id: number(site_id)?,
            post_id: number(post_id)?,
        }),
        ["read" | "reader", "feeds", feed_id, "posts", post_id] => Some(ReaderLink::FeedPost {
            feed_id: number(feed_id)?,
            post_id: number(post_id)?,
        }),
        _ => None,
    }
}

/// A post's address on a WordPress.com site: `/2024/05/31/a-post/`.
fn is_wordpress_com_post(host: &str, segments: &[&str]) -> bool {
    let is_wordpress_com = host == "wordpress.com" || host.ends_with(".wordpress.com");
    let is_number =
        |segment: &str| !segment.is_empty() && segment.bytes().all(|b| b.is_ascii_digit());
    is_wordpress_com
        && matches!(segments, [year, month, day, _] if is_number(year) && is_number(month) && is_number(day))
}

fn is_youtube_video(link: &Url, host: &str, segments: &[&str]) -> bool {
    match host {
        "youtube.com" | "www.youtube.com" | "m.youtube.com" => {
            (matches!(segments, ["watch"]) && link.query_pairs().any(|(name, _)| name == "v"))
                || matches!(segments, ["embed" | "shorts" | "live", _])
        }
        "youtu.be" => segments.len() == 1,
        _ => false,
    }
}

fn query_number(link: &Url, name: &str) -> Option<u64> {
    link.query_pairs()
        .find(|(key, _)| key == name)
        .and_then(|(_, value)| value.parse().ok())
}

/// Whether `link` is `page`, apart from its fragment, its scheme, `www.` and a trailing slash.
fn is_same_page(link: &Url, page: &Url) -> bool {
    let host = |url: &Url| {
        url.host_str()
            .map(|host| host.trim_start_matches("www.").to_ascii_lowercase())
    };
    host(link) == host(page)
        && link.path().trim_end_matches('/') == page.path().trim_end_matches('/')
        && link.query() == page.query()
}

fn external(link: &Url) -> ReaderLink {
    ReaderLink::External {
        url: link.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    const POST: &str = "https://example.com/2024/05/31/a-post/";

    fn url(url: &str) -> String {
        url.to_owned()
    }

    #[rstest]
    #[case::anchor_in_the_post("https://example.com/2024/05/31/a-post/#fn-1", ReaderLink::Anchor { id: url("fn-1") })]
    #[case::anchor_without_a_trailing_slash("http://www.example.com/2024/05/31/a-post#notes", ReaderLink::Anchor { id: url("notes") })]
    #[case::anchor_on_the_reader_page("https://wordpress.com/reader#fn-1", ReaderLink::Anchor { id: url("fn-1") })]
    #[case::relative_anchor("#caf%C3%A9", ReaderLink::Anchor { id: url("café") })]
    #[case::comment("https://example.com/2024/05/31/a-post/#comment-50484", ReaderLink::Comment { comment_id: 50484 })]
    #[case::anchor_in_another_post(
        "https://example.com/2024/06/01/another-post/#fn-1",
        ReaderLink::External { url: url("https://example.com/2024/06/01/another-post/#fn-1") }
    )]
    #[case::tag("https://wordpress.com/tag/photography", ReaderLink::Tag { slug: url("photography") })]
    #[case::site("https://wordpress.com/read/blogs/12345", ReaderLink::Site { site_id: 12345 })]
    #[case::feed("https://wordpress.com/reader/feeds/678", ReaderLink::Feed { feed_id: 678 })]
    #[case::site_post("https://wordpress.com/read/blogs/12345/posts/9", ReaderLink::SitePost { site_id: 12345, post_id: 9 })]
    #[case::feed_post("https://wordpress.com/read/feeds/678/posts/90", ReaderLink::FeedPost { feed_id: 678, post_id: 90 })]
    #[case::blog_preview("wordpress://blogpreview?blogId=12345", ReaderLink::Site { site_id: 12345 })]
    #[case::wordpress_com_post(
        "https://someone.wordpress.com/2024/05/31/hello/",
        ReaderLink::WordPressComPost { url: url("https://someone.wordpress.com/2024/05/31/hello/") }
    )]
    #[case::post_on_another_host(
        "https://example.org/2024/05/31/hello/",
        ReaderLink::External { url: url("https://example.org/2024/05/31/hello/") }
    )]
    #[case::mention("https://a8c.wordpress.com/mentions/someone/", ReaderLink::Mention { username: url("someone") })]
    #[case::image("https://example.com/wp-content/uploads/2024/05/photo-1024x768.JPG", ReaderLink::Image { url: url("https://example.com/wp-content/uploads/2024/05/photo-1024x768.JPG") })]
    #[case::image_with_a_query("https://i0.wp.com/example.com/a.webp?w=800", ReaderLink::Image { url: url("https://i0.wp.com/example.com/a.webp?w=800") })]
    #[case::heic("https://example.com/a.heic", ReaderLink::Image { url: url("https://example.com/a.heic") })]
    #[case::video_file("https://example.com/a.mp4", ReaderLink::Video { url: url("https://example.com/a.mp4") })]
    #[case::youtube("https://www.youtube.com/watch?v=abc", ReaderLink::Video { url: url("https://www.youtube.com/watch?v=abc") })]
    #[case::youtube_short_link("https://youtu.be/abc", ReaderLink::Video { url: url("https://youtu.be/abc") })]
    #[case::youtube_channel("https://www.youtube.com/@someone", ReaderLink::External { url: url("https://www.youtube.com/@someone") })]
    #[case::pdf("https://example.com/menu.pdf", ReaderLink::File { url: url("https://example.com/menu.pdf") })]
    #[case::podcast("https://example.com/episode.mp3", ReaderLink::File { url: url("https://example.com/episode.mp3") })]
    #[case::story("https://example.com/?wp-story=1", ReaderLink::Story { url: url("https://example.com/?wp-story=1") })]
    #[case::phone("tel:+1-555-0100", ReaderLink::Phone { number: url("+1-555-0100") })]
    #[case::text_message("sms:+15550100", ReaderLink::TextMessage { number: url("+15550100") })]
    #[case::email("mailto:someone@example.com", ReaderLink::Email { address: url("someone@example.com") })]
    #[case::page("https://example.org/about/", ReaderLink::External { url: url("https://example.org/about/") })]
    #[case::relative_link("/about/", ReaderLink::External { url: url("https://example.com/about/") })]
    #[case::other_scheme("slack://open", ReaderLink::External { url: url("slack://open") })]
    fn classifies_links(#[case] link: &str, #[case] expected: ReaderLink) {
        assert_eq!(reader_link(link, Some(POST.to_owned())), expected);
    }

    #[test]
    fn needs_the_post_to_know_its_anchors() {
        assert_eq!(
            reader_link("https://example.com/2024/05/31/a-post/#fn-1", None),
            ReaderLink::External {
                url: url("https://example.com/2024/05/31/a-post/#fn-1")
            }
        );
        assert_eq!(
            reader_link("#fn-1", None),
            ReaderLink::Anchor { id: url("fn-1") }
        );
    }
}
