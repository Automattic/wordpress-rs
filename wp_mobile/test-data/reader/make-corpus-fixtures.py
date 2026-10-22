#!/usr/bin/env python3
"""
Makes anonymized test posts from a corpus of real Reader API responses.

The corpus is real people's posts, so it stays out of this repository. This picks the fewest posts
that cover every feature the corpus's detector found, and anonymizes them so they can be committed:

- Text keeps its shape but not its words. Each word becomes another word of the same length, case
  and script, the same way everywhere, so an excerpt still matches its post and a mention still
  matches its username.
- Sites become `*.example` hosts, keeping their kind: `*.files.wordpress.com` stays
  `*.files.wordpress.com`. Embed and CDN hosts, like YouTube or Photon, stay as they are, but the
  IDs in their URLs don't.
- URLs keep their scheme, file extensions and WordPress's size suffixes (`-1024x768`), and ids keep
  matching the `#fragment`s that link to them.

Usage:

    python3 -I make-corpus-fixtures.py <corpus/> <manifest.csv> <output directory> [--count N]
"""

import argparse
import csv
import hashlib
import html
import json
import os
import re
import sys
from html.parser import HTMLParser

SALT = "wordpress-rs reader corpus"

# --- Words ---------------------------------------------------------------------------------------

LOREM = """a ab ad at et ex id in is ne non qui quo sed sit ut vel cum eum hic iam ius nam per
porro quia ante esse fuga illo ipsa ipsum iste nemo nisi odio quae quam sint sunt unde aut amet
autem culpa dicta dolor eaque eius enim error harum illum ipsam irure lorem magna magni minus
modi natus nihil nobis nulla omnis optio quas quasi quod rerum saepe sequi sunt tempor ullam
velit vitae alias animi aperiam beatae commodi corporis dolores dolorem eveniet facere fugiat
impedit ipsum labore laborum maiores minima nesciunt numquam officia pariatur possimus
quaerat quisquam ratione rerum soluta tempora tenetur totam veniam veritatis voluptas
accusamus architecto asperiores consectetur cupiditate deleniti dignissimos distinctio
doloremque exercitationem explicabo laboriosam molestiae necessitatibus perferendis
perspiciatis praesentium provident quibusdam recusandae reiciendis repellendus
reprehenderit similique voluptatem voluptatibus""".split()
LOREM_BY_LENGTH = {}
for word in LOREM:
    LOREM_BY_LENGTH.setdefault(len(word), []).append(word)

# Letters to stand in for each script's, so text keeps its direction and script.
SCRIPTS = [
    (0x0600, 0x06FF, "ابتثجحخدذرزسشصضطظعغفقكلمنهوي"),
    (0x0590, 0x05FF, "אבגדהוזחטיכלמנסעפצקרשת"),
    (0x0400, 0x04FF, "абвгдежзиклмнопрстуфхцчшэюя"),
    (0x0370, 0x03FF, "αβγδεζηθικλμνξοπρστυφχψω"),
    (0x0900, 0x097F, "कखगघचछजझटठडढतथदधनपफबभमयरलवशसह"),
    (0x0E00, 0x0E7F, "กขคงจฉชซญดตถทธนบปผพฟภมยรลวศสหอฮ"),
    (0x3040, 0x309F, "あいうえおかきくけこさしすせそたちつてとなにぬねの"),
    (0x30A0, 0x30FF, "アイウエオカキクケコサシスセソタチツテトナニヌネノ"),
    (0xAC00, 0xD7AF, "가나다라마바사아자차카타파하"),
    (0x4E00, 0x9FFF, "的一是不了人我在有他这中大来上国个到说们为子和你地出道也时年得就那要下以生会自着去之过家学对可"),
]


def digest(*parts):
    return hashlib.sha256((SALT + "\0" + "\0".join(parts)).encode()).digest()


def pick(alphabet, seed, index):
    return alphabet[digest(seed, str(index))[0] % len(alphabet)]


def map_word(word):
    """Another word with the same length, case and script, the same way every time."""
    base = word.lower()
    if base.isascii() and base.isalpha():
        candidates = LOREM_BY_LENGTH.get(len(base))
        if candidates:
            mapped = candidates[digest(base)[0] % len(candidates)]
        else:
            mapped = "".join(pick("abcdefghijklmnopqrstuvwxyz", base, i) for i in range(len(base)))
    else:
        mapped = "".join(map_char(c, base, i) for i, c in enumerate(base))
    # Keep the case of each letter.
    return "".join(m.upper() if o.isupper() else m for o, m in zip(word, mapped))


def map_char(c, seed, index):
    if c.isdigit() and c.isascii():
        return pick("0123456789", seed, index)
    if c.isascii() and c.isalpha():
        return pick("abcdefghijklmnopqrstuvwxyz", seed, index)
    code = ord(c)
    for start, end, alphabet in SCRIPTS:
        if start <= code <= end:
            return pick(alphabet, seed, index) if c.isalpha() else c
    if c.isalpha():
        return pick("abcdefghijklmnopqrstuvwxyz", seed, index)
    return c


# Chinese, Japanese and Korean don't put spaces between words, so each character is a word.
# Otherwise a tag in the middle of a sentence would change how it maps, and an excerpt would no
# longer match its post.
CJK = "\u3040-\u30ff\u3400-\u9fff\uac00-\ud7af\uf900-\ufaff"
WORD = re.compile(rf"[{CJK}]|[^\W_{CJK}]+", re.UNICODE)


def map_words(text):
    return WORD.sub(lambda m: map_word(m.group(0)), text)


def map_text(text):
    """Text, with any URLs in it still URLs."""
    parts, last = [], 0
    for match in re.finditer(r"https?://[^\s<>\"']+", text):
        parts.append(map_words(text[last:match.start()]))
        parts.append(map_url(match.group(0)))
        last = match.end()
    parts.append(map_words(text[last:]))
    return "".join(parts)


# Names map one to one, unlike text, so two ids or usernames never end up the same.
NAMES = {}
NAMES_USED = set()


def map_name_word(word):
    if word in NAMES:
        return NAMES[word]
    attempt = 0
    while True:
        seed = f"{word}\0{attempt}"
        mapped = "".join(map_char(c, seed, i) for i, c in enumerate(word))
        if mapped not in NAMES_USED:
            NAMES[word] = mapped
            NAMES_USED.add(mapped)
            return mapped
        attempt += 1


def map_name(name):
    """An id, a username or a URL fragment: words map one to one, and separators stay."""
    return WORD.sub(lambda m: map_name_word(m.group(0)), name)


# --- URLs ----------------------------------------------------------------------------------------

# Embed providers and CDNs: their hosts stay, because the cleanup and the stylesheet care which
# provider an embed or script comes from.
KEPT_HOST_SUFFIXES = """youtube.com youtu.be youtube-nocookie.com ytimg.com vimeo.com vimeocdn.com x.com
twitter.com twimg.com instagram.com cdninstagram.com facebook.com facebook.net fbcdn.net spotify.com
soundcloud.com sndcdn.com tiktok.com tumblr.com reddit.com redditmedia.com google.com googleapis.com
gstatic.com googlesyndication.com googleusercontent.com amazon-adsystem.com flickr.com staticflickr.com
gettyimages.com scribd.com slideshare.net bandcamp.com ted.com dailymotion.com giphy.com pinterest.com
pinimg.com mailchimp.com list-manage.com gravatar.com wp.me videopress.com w.org archive.org
pocketcasts.com speakerdeck.com cloudup.com loom.com kickstarter.com bsky.app threads.net codepen.io
amzn.to amazon.com amazon.co.uk valuecommerce.com 5min.com marinetraffic.com beehiiv.com jetpack.com
wordpress.org w3.org schema.org wp.com""".split()
KEPT_WORDPRESS_COM_HOSTS = {
    "wordpress.com", "www.wordpress.com", "en.wordpress.com", "video.wordpress.com",
    "v0.wordpress.com", "public-api.wordpress.com", "widgets.wp.com",
}
# Path words that are structure, not content.
KEPT_SEGMENTS = set("""embed embeds embed.js embeds.js widgets.js sdk.js post.js platform.js
widgets en_US en status p reel tv watch v video videos player track playlist album episode show
artist maps document d presentation file preview pubembed view s i images core emoji wp-content
uploads plugins themes mshots v1 js next videopress-iframe.js pagead adsbygoogle.js files media
assets svg 72x72 photos sets wp-includes blocks css img static""".split())
SIZE_SUFFIX = re.compile(r"^(.*?)((?:-\d+x\d+)?(?:-scaled)?)$")
KEPT_QUERY_KEYS = {"w", "h", "resize", "fit", "ssl", "quality", "zoom", "crop", "strip", "is-pending-load", "m", "ver"}
URL_IN_TEXT = re.compile(r"(?:https?:)?//[^\s\"'<>()\\]+", re.I)


def is_kept_host(host):
    host = host.lower()
    if host in KEPT_WORDPRESS_COM_HOSTS:
        return True
    return any(host == suffix or host.endswith("." + suffix) for suffix in KEPT_HOST_SUFFIXES)


# Kept hosts whose subdomains are their users' names, like `<blog>.tumblr.com`.
USER_SUBDOMAIN_SUFFIXES = ("tumblr.com", "bandcamp.com", "list-manage.com", "beehiiv.com")
SERVICE_SUBDOMAINS = {"www", "assets", "secure", "embed", "api", "static", "media", "platform"}


def map_host(host):
    host = host.lower()
    port = ""
    if ":" in host:
        host, port = host.split(":", 1)
        port = ":" + port
    for suffix in USER_SUBDOMAIN_SUFFIXES:
        if host.endswith("." + suffix):
            user, rest = host.split(".", 1)
            if user not in SERVICE_SUBDOMAINS and not user.isdigit():
                return "site-" + digest(host).hex()[:6] + "." + rest + port
    if is_kept_host(host):
        return host + port
    for kind in ("files.wordpress.com", "wordpress.com", "wpcomstaging.com", "blog"):
        if host.endswith("." + kind):
            return "site-" + digest(host).hex()[:6] + "." + kind + port
    www = "www." if host.startswith("www.") else ""
    return www + "site-" + digest(host.removeprefix("www.")).hex()[:6] + ".example" + port


def map_segment(segment, kept_host):
    if not segment or segment in KEPT_SEGMENTS or re.fullmatch(r"\d{4}|\d{2}", segment):
        return segment
    stem, dot, extension = segment.rpartition(".")
    if not dot or "/" in extension or len(extension) > 5:
        stem, extension, dot = segment, "", ""
    match = SIZE_SUFFIX.match(stem)
    name, size = match.group(1), match.group(2)
    return map_name(name) + size + dot + extension.lower() if extension else map_name(name) + size


def map_path(path, kept_host):
    return "/".join(map_segment(segment, kept_host) for segment in path.split("/"))


LONG_NUMBER = re.compile(r"\d{6,}")


def map_long_numbers(text):
    """IDs and timestamps, like a blog's ID. Sizes and years are shorter, so they stay."""
    return LONG_NUMBER.sub(lambda m: map_name_word(m.group(0)), text)


def map_query(query):
    if not query:
        return query
    parts = []
    for pair in query.split("&"):
        key, equals, value = pair.partition("=")
        if key.lower() in KEPT_QUERY_KEYS:
            parts.append(pair)
        elif re.fullmatch(r"[\d.,%C]*", value):
            parts.append(key + equals + map_long_numbers(value))
        else:
            parts.append(key + equals + map_name(value))
    return "&".join(parts)


def map_url(url):
    stripped = url.strip()
    if not stripped:
        return url
    lower = stripped.lower()
    if lower.startswith(("data:", "javascript:")):
        return stripped
    if lower.startswith(("tel:", "sms:")):
        return re.sub(r"\d", lambda m: map_char(m.group(0), stripped, m.start()), stripped)
    if lower.startswith("mailto:"):
        return "mailto:someone@example.com"
    match = re.match(r"^([a-z][a-z0-9+.-]*:)?(//([^/?#]*))?([^?#]*)(\?[^#]*)?(#.*)?$", stripped, re.I)
    if not match:
        return map_name(stripped)
    scheme, authority, host, path, query, fragment = match.groups()
    out = scheme or ""
    kept = False
    if authority is not None:
        kept = is_kept_host(host.split(":")[0]) if host else False
        out += "//" + (map_host(host) if host else "")
        # Photon serves another site's image: its first path segment is that site's host.
        if host and re.fullmatch(r"i\d+\.wp\.com", host.lower()) and path.count("/") >= 2:
            _, origin, rest = path.split("/", 2)
            path = "/" + map_host(origin) + "/" + map_path(rest, False)
            out += path
            path = None
    if path is not None:
        out += map_path(path, kept)
    if query:
        out += "?" + map_query(query[1:])
    if fragment:
        out += "#" + map_name(fragment[1:])
    return out


def map_srcset(srcset):
    candidates = []
    for candidate in re.split(r",\s+", srcset.strip()):
        url, _, descriptors = candidate.strip().partition(" ")
        candidates.append((map_url(url) + (" " + descriptors.strip() if descriptors.strip() else "")))
    return ", ".join(candidates)


# A domain without a scheme, like the ones in some plugins' JSON.
DOMAIN = re.compile(r"(?<![\w./@-])([a-z0-9][a-z0-9-]*(?:\.[a-z0-9-]+)*\.([a-z]{2,12}))(?=[/\\\"'\s,;)]|$)", re.I)
FILE_EXTENSIONS = set("""jpg jpeg png gif webp avif heic svg ico bmp tif tiff mp4 mov webm m4v mp3 m4a ogg wav
pdf doc docx js css json html htm php xml txt zip""".split())


def map_urls_in(text):
    # JSON escapes the slashes in URLs.
    escaped = "\\/" in text
    if escaped:
        text = text.replace("\\/", "/")
    text = URL_IN_TEXT.sub(lambda m: map_url(m.group(0)), text)
    return text.replace("/", "\\/") if escaped else text


UUID = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}", re.I)
# A content hash, like the CID that names one Bluesky post: long, and letters mixed with digits.
HASH = re.compile(r"\b(?=[a-z0-9]*[0-9])(?=[a-z0-9]*[a-z])[a-z0-9]{20,}\b", re.I)


def map_data(text):
    """An attribute nothing else covers, like a plugin's JSON: its URLs, domains and IDs."""
    text = map_long_numbers(map_urls_in(text))
    text = UUID.sub(lambda m: map_name(m.group(0)), text)
    text = HASH.sub(lambda m: map_name(m.group(0)), text)
    def replace(match):
        domain, top_level = match.group(1), match.group(2).lower()
        if top_level in FILE_EXTENSIONS or is_kept_host(domain):
            return domain
        return map_host(domain)

    return DOMAIN.sub(replace, text)


# Keys whose values are a person's words or names, like `title` or `fieldLabel`, wherever they are in
# some JSON.
TEXT_KEYS = re.compile(r"(name|label|title|caption|alt|description|author|byline|credit|copyright|user|email|text|placeholder|value|content|message|summary)$", re.I)


def map_json(value, key=""):
    """Some JSON, like a block's or a plugin's attributes: its strings and long numbers, but not its
    keys or its shape."""
    if isinstance(value, dict):
        return {k: map_json(v, k) for k, v in value.items()}
    if isinstance(value, list):
        return [map_json(v, key) for v in value]
    if isinstance(value, bool) or value is None:
        return value
    if isinstance(value, int):
        return int(map_long_numbers(str(value)))
    if isinstance(value, float):
        return value
    if URL_IN_TEXT.fullmatch(value.strip()):
        return map_url(value)
    if key.lower().endswith("srcset"):
        return map_srcset(value)
    # A media query and widths, like `(max-width: 460px) 576w, 120vw`.
    if key.lower() == "sizes":
        return value
    # A hash, like a form's.
    if re.fullmatch(r"[0-9a-f]{32,}", value):
        return map_name(value)
    # An identifier, like `"type":"image"`, unless it's under a key for words or names. It can
    # still be a domain or an ID.
    if not TEXT_KEYS.search(key) and re.fullmatch(r"[a-z0-9_./-]*", value):
        return map_data(value)
    return map_text(value)


def map_json_text(text):
    """Text holding JSON, mapped with `map_json`, or `None` if it isn't JSON."""
    stripped = text.strip()
    if not stripped.startswith(("{", "[")):
        return None
    try:
        value = json.loads(stripped)
    except ValueError:
        return None
    mapped = json.dumps(map_json(value), ensure_ascii=False, separators=(",", ":"))
    return mapped.replace("/", "\\/") if "\\/" in stripped else mapped


# --- HTML ----------------------------------------------------------------------------------------

URL_ATTRIBUTES = set("""href src poster action cite data-orig-file data-large-file data-medium-file
data-permalink data-src data-lazy-src data-url data-href data-wpcom-embed-url data-instgrm-permalink
data-original background longdesc data-full-url data-link data-orig-src formaction data-saferedirecturl
data-hovercard""".split())
SRCSET_ATTRIBUTES = {"srcset", "data-srcset", "data-lazy-srcset"}
TEXT_ATTRIBUTES = set("""alt title aria-label placeholder data-image-title data-title
data-caption data-name value label data-text data-author content summary data-byline-name""".split())
HTML_TEXT_ATTRIBUTES = {"data-image-caption", "data-image-description"}
NAME_ATTRIBUTES = set("""id name for aria-describedby aria-labelledby aria-controls data-fn headers data-username
data-id-attribute data-ob-template""".split())
ATTRIBUTE = re.compile(r"""([^\s"'<>/=]+)(\s*=\s*)("[^"]*"|'[^']*'|[^\s"'=<>`]+)""")
EMPTY_IMAGE_META = (
    '{"aperture":"0","credit":"","camera":"","caption":"","created_timestamp":"0",'
    '"copyright":"","focal_length":"0","iso":"0","shutter_speed":"0","title":"","orientation":"0"}'
)


def map_attribute(name, value):
    name = name.lower()
    if name in URL_ATTRIBUTES:
        return map_url(value)
    if name in SRCSET_ATTRIBUTES:
        return map_srcset(value)
    if name in TEXT_ATTRIBUTES:
        return map_urls_in(map_text(value)) if not URL_IN_TEXT.fullmatch(value.strip()) else map_url(value)
    if name in HTML_TEXT_ATTRIBUTES:
        return anonymize_html(value)
    if name in NAME_ATTRIBUTES:
        return map_name(value)
    if name == "data-image-meta":
        return EMPTY_IMAGE_META
    if name == "class":
        return map_long_numbers(value)
    # Credentials, like a Map block's Mapbox key, and oEmbed's per-embed secrets.
    if re.search(r"key|token|secret|nonce|hash", name):
        return map_name(value)
    # Interactivity API directives name code, like `state.isOpen`, not data.
    if name.startswith("data-wp-") and name != "data-wp-context":
        return map_long_numbers(value)
    if name.startswith("data-") or "{" in value:
        mapped = map_json_text(value)
        return mapped if mapped is not None else map_data(value)
    return map_long_numbers(map_urls_in(value))


def map_tag(raw):
    def replace(match):
        name, equals, quoted = match.groups()
        quote = quoted[0] if quoted[0] in "\"'" else ""
        value = quoted[1:-1] if quote else quoted
        mapped = map_attribute(name, html.unescape(value))
        escaped = html.escape(mapped, quote=False)
        escaped = escaped.replace('"', "&quot;") if quote != "'" else escaped.replace("'", "&#39;")
        return f'{name}{equals}{quote or chr(34)}{escaped}{quote or chr(34)}'

    # The tag name stays; only attributes after it change.
    head = re.match(r"<\s*[^\s/>]+", raw)
    return raw[: head.end()] + ATTRIBUTE.sub(replace, raw[head.end():]) if head else raw


class Anonymizer(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=False)
        self.out = []
        self.raw_text = None

    def handle_starttag(self, tag, attrs):
        self.out.append(map_tag(self.get_starttag_text()))
        if tag in ("script", "style"):
            self.raw_text = tag

    def handle_startendtag(self, tag, attrs):
        self.out.append(map_tag(self.get_starttag_text()))

    def handle_endtag(self, tag):
        self.raw_text = None
        self.out.append(f"</{tag}>")

    def handle_data(self, data):
        if self.raw_text == "script":
            self.out.append("/* anonymized */" if data.strip() else data)
        elif self.raw_text == "style":
            self.out.append(map_long_numbers(map_urls_in(data)))
        else:
            self.out.append(map_text(data))

    def handle_entityref(self, name):
        self.out.append(f"&{name};")

    def handle_charref(self, name):
        self.out.append(f"&#{name};")

    def handle_comment(self, data):
        # Block comments keep their block's name. Their attributes are JSON, like an embed's URL.
        block = re.fullmatch(r"(\s*/?wp:[\w/-]+\s*)(.*?)(\s*/?\s*)", data, re.S)
        if not block:
            self.out.append("<!-- anonymized -->")
            return
        name, attributes, end = block.groups()
        if attributes:
            attributes = map_json_text(attributes) or "{}"
        self.out.append(f"<!--{name}{attributes}{end}-->")

    def handle_decl(self, decl):
        self.out.append(f"<!{decl}>")

    def handle_pi(self, data):
        self.out.append(f"<?{data}>")

    def unknown_decl(self, data):
        self.out.append(f"<![{data}]>")


def anonymize_html(text):
    parser = Anonymizer()
    parser.feed(text)
    parser.close()
    return "".join(parser.out)


# --- Picking posts -------------------------------------------------------------------------------

# Embed providers the detector names. It names an embedded WordPress site after the site, so any
# other name is a WordPress site, and isn't worth keeping (or committing).
EMBED_PROVIDERS = set("""youtube instagram twitter x videopress soundcloud google-maps spotify bandcamp
vimeo giphy tiktok amazon slideshare google-docs ted speakerdeck facebook pocket-casts reddit
pinterest dailymotion internet-archive scribd cloudup codepen tumblr crowdsignal google-drive
google-calendar flourish knight-lab wordpress-tv issuu flickr mixcloud apple-podcasts loom
kickstarter github embed embed-handler gravatar""".split())


def embed_provider(name):
    return name if name in EMBED_PROVIDERS else "wordpress-site"


def features_of(row):
    features = set(row["features"].split())
    features |= {"embed:" + embed_provider(e) for e in row["embeds"].split() if e}
    features |= {"script:" + h for h in row["script_hosts"].split() if h}
    features.add("site:" + row["site_type"])
    for language in row["language"].split(","):
        features.add("language:" + language)
    return features


def pick_posts(rows, sizes, count, must):
    """The fewest small posts that cover every feature, with the ones in `must` first."""
    uncovered = set().union(*(features_of(row) for row in rows))
    chosen = []
    by_id = {row["id"]: row for row in rows}
    for post_id in must:
        if post_id in by_id:
            chosen.append(post_id)
            uncovered -= features_of(by_id[post_id])
    candidates = [row for row in rows if row["id"] not in chosen and sizes[row["id"]] < 80_000]
    while uncovered and len(chosen) < count:
        best = max(
            candidates,
            key=lambda row: (len(features_of(row) & uncovered) / (sizes[row["id"]] / 1024 + 4), -sizes[row["id"]]),
        )
        gained = features_of(best) & uncovered
        if not gained:
            break
        chosen.append(best["id"])
        candidates.remove(best)
        uncovered -= gained
    return chosen, uncovered


def fixture_name(features, used):
    # Name a post after its rarest feature, so the name says why it's there.
    for feature in features:
        name = re.sub(r"[^a-z0-9]+", "-", feature.lower()).strip("-")
        if name and name not in used:
            used.add(name)
            return name
    name = f"post-{len(used)}"
    used.add(name)
    return name


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("corpus")
    parser.add_argument("manifest")
    parser.add_argument("output")
    parser.add_argument("--count", type=int, default=60)
    parser.add_argument("--must", default="", help="Comma-separated post IDs to always include")
    parser.add_argument("--map", help="Where to write which fixture came from which post (keep it private)")
    args = parser.parse_args()

    rows = list(csv.DictReader(open(args.manifest)))
    posts = {}
    for row in rows:
        path = os.path.join(args.corpus, row["id"] + ".json")
        if os.path.exists(path):
            posts[row["id"]] = json.load(open(path))
    rows = [row for row in rows if row["id"] in posts]
    sizes = {row["id"]: len(posts[row["id"]]["fields"].get("content") or "") for row in rows}
    must = [m for m in args.must.split(",") if m]

    chosen, uncovered = pick_posts(rows, sizes, args.count, must)
    frequency = {}
    for row in rows:
        for feature in features_of(row):
            frequency[feature] = frequency.get(feature, 0) + 1

    os.makedirs(args.output, exist_ok=True)
    used, mapping = set(), {}
    by_id = {row["id"]: row for row in rows}
    for post_id in chosen:
        post, row = posts[post_id], by_id[post_id]
        features = sorted(features_of(row), key=lambda f: (frequency[f], f))
        name = fixture_name([f for f in features if not f.startswith(("site:", "language:"))], used)
        fields = post["fields"]
        fixture = {
            "source": {
                "tier": "B, anonymized from the private corpus",
                "site_type": post["source"].get("site_type"),
                "p2": post["source"].get("p2", False),
            },
            "fields": {
                "title": map_text(fields.get("title") or ""),
                "URL": map_url(fields.get("URL") or ""),
                "content": anonymize_html(fields.get("content") or ""),
                "excerpt": anonymize_html(fields.get("excerpt") or ""),
                "featured_image": map_url(fields.get("featured_image") or ""),
            },
            "features": sorted(set(post.get("features", [])) | {f for f in features_of(row) if ":" in f}),
        }
        with open(os.path.join(args.output, name + ".json"), "w") as file:
            json.dump(fixture, file, ensure_ascii=False, indent=1)
            file.write("\n")
        mapping[name] = post_id

    if args.map:
        json.dump(mapping, open(args.map, "w"), indent=1)
    total = sum(os.path.getsize(os.path.join(args.output, n + ".json")) for n in mapping)
    print(f"{len(chosen)} posts, {total // 1024} KB. Features not covered: {sorted(uncovered) or 'none'}")


if __name__ == "__main__":
    sys.exit(main())
