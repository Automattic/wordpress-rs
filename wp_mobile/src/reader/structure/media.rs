//! Real, public media to show in a structure's stand-ins' place, so a preview's images load and its
//! embeds play. Only the preview uses it, in [`with_real_media`]: structures themselves never hold
//! anything real.
//!
//! A stand-in maps to an item by its number, so the same stand-in is always the same item, and a
//! post with a few of them gets a few different ones. An image keeps the size its structure says,
//! from its name's `-1024x768` or its query's `w` and `resize`, and gets a photo in the same shape.
//!
//! Every item was checked on 2026-10-21, with its provider's oEmbed endpoint or by loading it.
//! `media_library_is_still_there` checks them again.

use std::cell::Cell;

use super::url::{FileName, Key, QueryParam, QueryValue, Segment, Url};
use super::vocabulary::{KnownHost, PathWord, QueryKey};
use super::{TokenId, Vocabulary};

thread_local! {
    static REAL: Cell<bool> = const { Cell::new(false) };
}

/// Runs `render` with real media in the stand-ins' place.
pub(crate) fn with_real_media<T>(render: impl FnOnce() -> T) -> T {
    let before = REAL.with(|real| real.replace(true));
    let result = render();
    REAL.with(|real| real.set(before));
    result
}

pub(crate) fn is_on() -> bool {
    REAL.with(Cell::get)
}

// --- The library ----------------------------------------------------------------------------------

/// A photo on Wikimedia Commons, in the public domain or CC0, which Photon serves at any size.
/// Commons limits how often its own thumbnails load, so only Photon's are used.
struct Photo {
    path: &'static str,
    width: u32,
    height: u32,
}

const LANDSCAPE: &[Photo] = &[
    Photo {
        path: "0/01/Alone_in_the_unspoilt_wilderness_%28Unsplash%29.jpg",
        width: 3506,
        height: 2329,
    },
    Photo {
        path: "2/2c/Carrelet%2C_Esnandes%2C_Charente-Maritime%2C_august_2015.jpg",
        width: 4375,
        height: 2793,
    },
    Photo {
        path: "e/e3/Luxembourg_City_Hall_%2854540726309%29.jpg",
        width: 3868,
        height: 2430,
    },
];

const PORTRAIT: &[Photo] = &[
    Photo {
        path: "f/fb/Pavillon_Cour_des_Lions_Alhambra_Granada_Spain.jpg",
        width: 2155,
        height: 3012,
    },
    Photo {
        path: "a/a5/Portico_gallery_perspective_%C3%89cole_Militaire_Cour_d%27Honneur_Paris.jpg",
        width: 2853,
        height: 4348,
    },
];

const SQUARE: &[Photo] = &[
    Photo {
        path: "0/00/Crab_Nebula.jpg",
        width: 3864,
        height: 3864,
    },
    Photo {
        path: "3/3f/Hubble_NGC6530.jpg",
        width: 3880,
        height: 3845,
    },
];

/// Big Buck Bunny (10 minutes), Me at the zoo (19 seconds, 4:3), and a YouTube Developers video.
const YOUTUBE: &[&str] = &["aqz-KE-bpKQ", "jNQXAC9IVRw", "M7lc1UVf-VE"];
const VIMEO: &[&str] = &["1084537", "22439234"];
/// A tweet with only text, and two with a photo.
const TWEETS: &[(&str, &str)] = &[
    ("jack", "20"),
    ("BarackObama", "266031293945503744"),
    ("TheEllenShow", "440322224407314432"),
];
const INSTAGRAM: &[&str] = &["C", "BsOGulcndj-"];
const TIKTOKS: &[(&str, &str)] = &[("scout2015", "6718335390845095173")];
/// Bluesky's own posts: with images, with a long text and a quote, and with a video.
const BLUESKY: &[(&str, &str, &str)] = &[
    (
        "bsky.app",
        "did:plc:z72i7hdynmk6r22z27h6tvur",
        "3mxu3hsnebc2v",
    ),
    (
        "bsky.app",
        "did:plc:z72i7hdynmk6r22z27h6tvur",
        "3mxzez2m7wk2r",
    ),
    (
        "bsky.app",
        "did:plc:z72i7hdynmk6r22z27h6tvur",
        "3mxyyerrxak2m",
    ),
];
/// A small, a medium and a large gist: 175 bytes, 1.3 KB and 7.2 KB of code.
const GISTS: &[(&str, &str)] = &[
    ("octocat", "6cad326836d38bd3a7ae"),
    ("octocat", "2a6851cde24cdaf4b85b"),
    ("octocat", "1169854"),
];
const SPOTIFY: &[(&str, &str)] = &[
    ("track", "4uLU6hMCjMI75M1A2tKUQC"),
    ("album", "4LH4d3cOWNNsVw41Gqt2kv"),
    ("playlist", "37i9dQZF1DXcBWIGoYBM5M"),
];
const SOUNDCLOUD: &[(&str, &str)] = &[("forss", "flickermood")];
const REDDIT: &[(&str, &str, &str)] = &[(
    "IAmA",
    "z1c9z",
    "i_am_barack_obama_president_of_the_united_states",
)];
const FLICKR: &[(&str, &str)] = &[("nasawebbtelescope", "52211883534")];
/// A 1 MB, a 5 MB and a 62 MB video.
const VIDEOS: &[&str] = &[
    "https://test-videos.co.uk/vids/bigbuckbunny/mp4/h264/360/Big_Buck_Bunny_360_10s_1MB.mp4",
    "https://test-videos.co.uk/vids/bigbuckbunny/mp4/h264/720/Big_Buck_Bunny_720_10s_5MB.mp4",
    "https://archive.org/download/BigBuckBunny_124/Content/big_buck_bunny_720p_surround.mp4",
];
/// A few seconds of audio, and six minutes.
const AUDIO: &[&str] = &[
    "https://upload.wikimedia.org/wikipedia/commons/c/c8/Example.ogg",
    "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3",
];
const DOCUMENTS: &[&str] =
    &["https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf"];

/// What to check each item with: its provider's oEmbed endpoint, or the item itself.
pub(crate) fn checks() -> Vec<String> {
    let mut checks = Vec::new();
    for photo in LANDSCAPE.iter().chain(PORTRAIT).chain(SQUARE) {
        checks.push(photon(photo, "?w=640"));
    }
    for id in YOUTUBE {
        checks.push(format!(
            "https://www.youtube.com/oembed?format=json&url=https://www.youtube.com/watch?v={id}"
        ));
    }
    for id in VIMEO {
        checks.push(format!(
            "https://vimeo.com/api/oembed.json?url=https://vimeo.com/{id}"
        ));
    }
    for (user, id) in TWEETS {
        checks.push(format!(
            "https://publish.twitter.com/oembed?url=https://twitter.com/{user}/status/{id}"
        ));
    }
    for id in INSTAGRAM {
        checks.push(format!("https://www.instagram.com/p/{id}/embed/"));
    }
    for (user, id) in TIKTOKS {
        checks.push(format!(
            "https://www.tiktok.com/oembed?url=https://www.tiktok.com/@{user}/video/{id}"
        ));
    }
    for (handle, _, post) in BLUESKY {
        checks.push(format!(
            "https://embed.bsky.app/oembed?url=https://bsky.app/profile/{handle}/post/{post}"
        ));
    }
    for (owner, id) in GISTS {
        checks.push(format!("https://gist.github.com/{owner}/{id}.json"));
    }
    for (kind, id) in SPOTIFY {
        checks.push(format!(
            "https://open.spotify.com/oembed?url=https://open.spotify.com/{kind}/{id}"
        ));
    }
    for (user, track) in SOUNDCLOUD {
        checks.push(format!(
            "https://soundcloud.com/oembed?format=json&url=https://soundcloud.com/{user}/{track}"
        ));
    }
    for (subreddit, id, slug) in REDDIT {
        checks.push(format!(
            "https://www.reddit.com/oembed?url=https://www.reddit.com/r/{subreddit}/comments/{id}/{slug}/"
        ));
    }
    for (user, id) in FLICKR {
        checks.push(format!(
            "https://www.flickr.com/services/oembed/?format=json&url=https://www.flickr.com/photos/{user}/{id}/"
        ));
    }
    checks.extend(
        VIDEOS
            .iter()
            .chain(AUDIO)
            .chain(DOCUMENTS)
            .map(|url| (*url).to_owned()),
    );
    checks
}

// --- Stand-ins to media ---------------------------------------------------------------------------

fn pick<T>(items: &[T], number: u32) -> &T {
    &items[number as usize % items.len()]
}

fn photon(photo: &Photo, query: &str) -> String {
    format!(
        "https://i0.wp.com/upload.wikimedia.org/wikipedia/commons/{}{query}",
        photo.path
    )
}

/// The size a query asks for: Photon's `resize` or `fit`, or `w` and `h`.
fn query_size(query: &[QueryParam]) -> (Option<u32>, Option<u32>) {
    let number = |value: &QueryValue| match value {
        QueryValue::Number(number) => number.as_str().split('.').next()?.parse().ok(),
        _ => None,
    };
    let (mut width, mut height) = (None, None);
    for param in query {
        match (&param.key, &param.value) {
            (Key::Known(QueryKey::W), value) => width = number(value),
            (Key::Known(QueryKey::H), value) => height = number(value),
            (Key::Known(QueryKey::Resize | QueryKey::Fit), QueryValue::Numbers(numbers)) => {
                if let [w, h, ..] = numbers.as_slice() {
                    width = w.as_str().parse().ok();
                    height = h.as_str().parse().ok();
                }
            }
            _ => {}
        }
    }
    (width, height)
}

/// A photo for an image, in the image's shape, and at its size.
fn image(number: u32, file: Option<&FileName>, query: &[QueryParam]) -> String {
    let (width, height) = match file.and_then(|file| file.size) {
        Some((width, height)) => (Some(width), Some(height)),
        None => query_size(query),
    };
    let photos = match (width, height) {
        (Some(width), Some(height)) if height * 100 > width * 115 => PORTRAIT,
        (Some(width), Some(height)) if width.abs_diff(height) * 100 < width * 15 => SQUARE,
        _ => LANDSCAPE,
    };
    let photo = pick(photos, number);
    match (width, height) {
        (Some(width), Some(height)) => photon(photo, &format!("?resize={width},{height}")),
        (Some(width), None) => photon(photo, &format!("?w={width}")),
        _ => {
            let width = photo.width.min(1024);
            photon(
                photo,
                &format!("?resize={width},{}", width * photo.height / photo.width),
            )
        }
    }
}

fn file(number: u32, name: &FileName) -> Option<String> {
    let extension = name.extension.as_str();
    let files = if matches!(
        extension,
        "mp4" | "m4v" | "mov" | "webm" | "ogv" | "3gp" | "avi" | "mkv" | "mpeg" | "mpg" | "wmv"
    ) {
        VIDEOS
    } else if matches!(
        extension,
        "mp3" | "m4a" | "wav" | "ogg" | "oga" | "flac" | "aac" | "wma"
    ) {
        AUDIO
    } else if matches!(
        extension,
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "rtf" | "txt" | "epub"
    ) {
        DOCUMENTS
    } else {
        return None;
    };
    Some((*pick(files, number)).to_owned())
}

/// The number that picks a provider's item: the first stand-in in its path, or in its query.
fn number(path: &[Segment], query: &[QueryParam]) -> u32 {
    path.iter()
        .find_map(|segment| match segment {
            Segment::StandIn(id, _) | Segment::File(id, _, _) => Some(id.0),
            _ => None,
        })
        .or_else(|| {
            query.iter().find_map(|param| match param.value {
                QueryValue::Other(id) => Some(id.0),
                _ => None,
            })
        })
        .unwrap_or(0)
}

fn has_word(path: &[Segment], word: PathWord) -> bool {
    path.contains(&Segment::Word(word))
}

fn provider(host: KnownHost, www: bool, path: &[Segment], query: &[QueryParam]) -> Option<String> {
    // A provider's own script, like TikTok's `embed.js`, stays, except a gist's, which is the gist.
    let is_script = path.iter().any(|segment| match segment {
        Segment::Script(_) => true,
        Segment::File(_, _, file) => matches!(file.extension.as_str(), "js" | "css"),
        _ => false,
    });
    if is_script && host.as_str() != "gist.github.com" {
        return None;
    }
    let n = number(path, query);
    let www = if www { "www." } else { "" };
    Some(match host.as_str() {
        "youtube.com" | "youtube-nocookie.com" => {
            let id = pick(YOUTUBE, n);
            let host = format!("{www}{}", host.as_str());
            if has_word(path, PathWord::Embed) {
                format!("https://{host}/embed/{id}")
            } else if has_word(path, PathWord::Shorts) {
                format!("https://{host}/shorts/{id}")
            } else {
                format!("https://{host}/watch?v={id}")
            }
        }
        "youtu.be" => format!("https://youtu.be/{}", pick(YOUTUBE, n)),
        "i.ytimg.com" => format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", pick(YOUTUBE, n)),
        "vimeo.com" => format!("https://vimeo.com/{}", pick(VIMEO, n)),
        "player.vimeo.com" => format!("https://player.vimeo.com/video/{}", pick(VIMEO, n)),
        "twitter.com" | "x.com" => {
            if !has_word(path, PathWord::Status) {
                return None;
            }
            let (user, id) = pick(TWEETS, n);
            format!("https://{www}{}/{user}/status/{id}", host.as_str())
        }
        "instagram.com" => format!("https://www.instagram.com/p/{}/", pick(INSTAGRAM, n)),
        "tiktok.com" => {
            let (user, id) = pick(TIKTOKS, n);
            format!("https://www.tiktok.com/@{user}/video/{id}")
        }
        "bsky.app" => {
            let (handle, _, post) = pick(BLUESKY, n);
            format!("https://bsky.app/profile/{handle}/post/{post}")
        }
        "gist.github.com" => {
            let (owner, id) = pick(GISTS, n);
            let extension = path.iter().find_map(|segment| match segment {
                Segment::File(_, _, file) => Some(file.extension.as_str()),
                _ => None,
            });
            match extension {
                Some(extension) => format!("https://gist.github.com/{owner}/{id}.{extension}"),
                None => format!("https://gist.github.com/{owner}/{id}"),
            }
        }
        "open.spotify.com" | "embed.spotify.com" => {
            let kind = [
                (PathWord::Track, "track"),
                (PathWord::Album, "album"),
                (PathWord::Playlist, "playlist"),
            ]
            .into_iter()
            .find(|(word, _)| has_word(path, *word))
            .map(|(_, kind)| kind);
            let (kind, id) = SPOTIFY
                .iter()
                .find(|(item, _)| Some(*item) == kind)
                .unwrap_or_else(|| pick(SPOTIFY, n));
            let embed = if has_word(path, PathWord::Embed) {
                "embed/"
            } else {
                ""
            };
            format!("https://open.spotify.com/{embed}{kind}/{id}")
        }
        "soundcloud.com" => {
            let (user, track) = pick(SOUNDCLOUD, n);
            format!("https://soundcloud.com/{user}/{track}")
        }
        "w.soundcloud.com" => {
            let (user, track) = pick(SOUNDCLOUD, n);
            format!("https://w.soundcloud.com/player/?url=https%3A//soundcloud.com/{user}/{track}")
        }
        "reddit.com" | "embed.reddit.com" => {
            let (subreddit, id, slug) = pick(REDDIT, n);
            format!("https://www.reddit.com/r/{subreddit}/comments/{id}/{slug}/")
        }
        "flickr.com" => {
            let (user, id) = pick(FLICKR, n);
            format!("https://www.flickr.com/photos/{user}/{id}/")
        }
        _ => return None,
    })
}

/// The real URL for a URL's stand-in, if the library has one.
pub(crate) fn real_url(url: &Url) -> Option<String> {
    match url {
        Url::Image {
            image: id,
            file,
            query,
            ..
        } => Some(image(id.0, Some(file), query)),
        Url::Photon { origin, query, .. } => match origin.as_ref() {
            Url::Image {
                image: id, file, ..
            } => {
                // Photon's own query says the size, if it has one, and its name otherwise.
                let size = query_size(query);
                let file = (size == (None, None)).then_some(file);
                Some(image(id.0, file, query))
            }
            _ => None,
        },
        Url::File { file, name, .. } => self::file(file.0, name),
        Url::Provider {
            host,
            www,
            path,
            query,
            ..
        } => provider(*host, *www, path, query),
        _ => None,
    }
}

/// A Bluesky post's `at://` URI, which its embed script loads the post from.
pub(crate) fn bluesky_uri(token: TokenId) -> String {
    let (_, did, post) = pick(BLUESKY, token.0);
    format!("at://{did}/app.bsky.feed.post/{post}")
}

/// A TikTok video's ID, which its embed script loads the video from.
pub(crate) fn tiktok_id(token: TokenId) -> &'static str {
    pick(TIKTOKS, token.0).1
}
