#!/usr/bin/env python3
"""Writes Documentation/reader-endpoints.md: every WordPress.com Reader endpoint the library has,
with the parameters it sends and what WordPress.com's public documentation declares for it.

    scripts/reader-endpoints-doc.py            Writes the document.
    scripts/reader-endpoints-doc.py --check    Fails if the document isn't what this would write.

It reads two things, both public:

- The library's own endpoint files (`wp_api/src/wp_com/endpoint/reader_*_endpoint.rs`) and the
  parameter types they name. What a parameter does is the comment on its field.
- WordPress.com's documentation of itself: `rest/v1.x/help` for the `rest/v1.x` endpoints, and
  the `wpcom/v2` route index. Neither needs a token. Pass `--public-docs <folder>` to read saved
  copies (`help-v1.1.json`, `help-v1.2.json`, `help-v1.3.json`, `index-v2.json`) instead of
  fetching them.
"""
import argparse
import json
import pathlib
import re
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
ENDPOINTS = ROOT / "wp_api/src/wp_com/endpoint"
TYPES = ROOT / "wp_api/src/wp_com"
DOCUMENT = ROOT / "Documentation/reader-endpoints.md"

NAMESPACES = {
    "RestV1_1": "/rest/v1.1",
    "RestV1_2": "/rest/v1.2",
    "RestV1_3": "/rest/v1.3",
    "V2": "/wpcom/v2",
}
PLACEHOLDERS = {
    "wp_com_site_id": "$site",
    "wp_com_site_identifier": "$site",
    "post_id": "$post_id",
    "comment_id": "$comment_id",
    "reader_feed_id": "$feed",
    "reader_feed_item_id": "$feed_item",
    "reader_tag_slug": "$tag",
    "reader_post_slug": "slug:$slug",
    "reader_list_owner": "$owner",
    "reader_list_slug": "$list",
    "wp_com_user_identifier": "$user",
}
GROUPS = {
    "reader_posts": "Streams of posts",
    "reader_post": "Single posts",
    "reader_sites": "Sites and feeds",
    "reader_tags": "Tags and the menu",
    "reader_users": "Users",
    "reader_saved": "Saved posts",
    "reader_seen": "Seen status",
    "reader_likes": "Likes",
    "reader_subscriptions": "Subscriptions",
    "reader_blocks": "Blocked sites",
    "reader_comments": "Comments",
}
# Declared for every `rest/v1.x` endpoint, so they're listed once, in the introduction.
COMMON = {"context", "http_envelope", "pretty", "meta", "fields", "callback"}
HELP_VERSIONS = ["1.3", "1.2", "1.1"]


def plain(text):
    """A doc comment as Markdown: rustdoc links become code, and `Self::x` becomes `x`."""
    text = re.sub(r"\[`(?:Self::)?([^`\]]+)`\](?:\([^)]*\))?", r"`\1`", text)
    return text.strip()


def comment_blocks(lines):
    """Comment lines as paragraphs and list items, each a string."""
    blocks, current = [], []
    for line in lines:
        if not line.strip():
            if current:
                blocks.append(" ".join(current))
                current = []
        elif line.lstrip().startswith("- "):
            if current:
                blocks.append(" ".join(current))
            current = [line.strip()]
        else:
            current.append(line.strip())
    if current:
        blocks.append(" ".join(current))
    return [plain(block) for block in blocks]


def parse_endpoint_file(path):
    source = path.read_text()
    enum = re.search(r"enum (\w+Request) \{\n(.*?)\n\}\n", source, re.S)
    requests, comment, attribute = [], [], None
    for line in enum.group(2).split("\n"):
        text = line.strip()
        if attribute is not None and not attribute.endswith(")]"):
            attribute += " " + text
        elif text.startswith("//"):
            comment.append(text[2:].strip())
        elif text.startswith("#[get(") or text.startswith("#[post("):
            attribute = text
        elif attribute and re.fullmatch(r"\w+,", text):
            method = "GET" if attribute.startswith("#[get") else "POST"
            requests.append(
                {
                    "variant": text[:-1],
                    "method": method,
                    "url": re.search(r'url = "([^"]+)"', attribute).group(1),
                    "params": (re.search(r"params = &(\w+)", attribute) or [None, None])[1],
                    "output": (re.search(r"output = (\w+)", attribute) or [None, None])[1],
                    "about": comment_blocks(comment),
                }
            )
            comment, attribute = [], None
        elif not text:
            comment = []

    body = re.search(r"fn namespace\(&self\) -> impl AsNamespace \{\n(.*?)\n    \}\n", source, re.S)
    body = re.sub(r"//[^\n]*", "", body.group(1))
    arms = re.findall(r"((?:\|?\s*Self::\w+\s*)+|_)\s*=>\s*\{?\s*WpComNamespace::(\w+)", body)
    default = None if arms else re.search(r"WpComNamespace::(\w+)", body).group(1)
    for request in requests:
        namespace = default
        for pattern, name in arms:
            if pattern.strip() == "_" or request["variant"] in re.findall(r"Self::(\w+)", pattern):
                namespace = name
                break
        request["namespace"] = NAMESPACES[namespace]
        request["enum"] = enum.group(1)
    return requests


def find_type(name):
    """The body of a struct or an enum of the library's WordPress.com types, and which it is."""
    for path in sorted(TYPES.glob("*.rs")):
        found = re.search(r"pub (struct|enum) %s \{\n(.*?)\n\}" % re.escape(name), path.read_text(), re.S)
        if found:
            return found.group(1), found.group(2)
    raise SystemExit(f"No type named {name}")


def parse_params(name):
    """The fields a parameter type puts on the wire, each with the comment on it."""
    kind, body = find_type(name)
    if kind == "enum":
        # Flattened into the request: each variant sends its own fields, and is described once.
        fields, comment = [], []
        for line in body.split("\n"):
            text = line.strip()
            if text.startswith("///"):
                comment.append(text[3:].strip())
            elif text.startswith("#[") or not text:
                continue
            else:
                variant = re.match(r"(\w+) \{(.*)", text)
                inner = variant.group(2) if variant else text
                about = comment_blocks(comment)
                for field, field_kind in re.findall(r"(\w+): ([\w<>]+)", inner):
                    fields.append({"name": field, "type": f"Option<{field_kind}>", "about": about})
                if text.endswith("},") or text == "},":
                    comment = []
        return fields
    fields, comment, wire, flatten = [], [], None, False
    for line in body.split("\n"):
        text = line.strip()
        if text.startswith("///"):
            comment.append(text[3:].strip())
        elif text.startswith("#["):
            renamed = re.search(r'(?:field_name\(|rename = )"([^"]+)"', text)
            wire = renamed.group(1) if renamed else wire
            flatten = flatten or "flatten" in text
        elif text.startswith("pub "):
            field, field_kind = re.fullmatch(r"pub (\w+): (.+),", text).groups()
            if flatten:
                # The fields of the type are sent in its place.
                inner = field_kind.removeprefix("Option<").removesuffix(">") if field_kind.startswith("Option<") else field_kind
                about = comment_blocks(comment)
                for nested in parse_params(inner):
                    optional = nested["type"] if nested["type"].startswith("Option<") else f"Option<{nested['type']}>"
                    fields.append({"name": nested["name"], "type": optional, "about": nested["about"] or about})
            else:
                fields.append({"name": wire or field, "type": field_kind, "about": comment_blocks(comment)})
            comment, wire, flatten = [], None, False
    return fields


def labeled(url):
    return re.sub(r"<(\w+)>", lambda match: PLACEHOLDERS[match.group(1)], url)


def shape(path):
    """A path with its variable parts made alike, to compare the library's with a documented one."""
    path = re.sub(r"\(\?P<\w+>[^)]*\)", "$", path)
    path = re.sub(r"slug:\$\w+|\$\w+", "$", path)
    return path.rstrip("/")


def fetch(url):
    request = urllib.request.Request(url, headers={"Accept": "application/json"})
    with urllib.request.urlopen(request, timeout=90) as response:
        return json.load(response)


def public_docs(folder):
    def load(name, url):
        if folder:
            return json.loads((pathlib.Path(folder) / name).read_text())
        return fetch(url)

    helps = {
        version: load(f"help-v{version}.json", f"https://public-api.wordpress.com/rest/v{version}/help")
        for version in HELP_VERSIONS
    }
    index = load("index-v2.json", "https://public-api.wordpress.com/wpcom/v2/")
    return helps, index


def declared_text(kind, about):
    """A publicly declared parameter's type and description, as one line."""
    if isinstance(about, dict):
        # The values a parameter takes, each with what it means.
        about = "One of: " + "; ".join(f"`{value}` ({meaning})" for value, meaning in about.items())
    about = re.sub(r"\s+", " ", re.sub(r"<[^>]+>", "", str(about or ""))).strip()
    kind = str(kind or "").strip().strip("()")
    return f"({kind}) {about}".strip() if kind else about


def declared_for(request, helps, index):
    """What the public documentation declares: (where, {name: description}), or None."""
    path = shape(labeled(request["url"]))
    if request["namespace"] == "/wpcom/v2":
        for route, entry in index["routes"].items():
            documented = shape(route.removeprefix("/wpcom/v2"))
            same = documented == path or (
                len(documented.split("/")) == len(path.split("/"))
                and all(a == b or a == "$" for a, b in zip(documented.split("/"), path.split("/")))
            )
            if not same:
                continue
            for endpoint in entry["endpoints"]:
                if request["method"] in endpoint["methods"]:
                    declared = {}
                    # A route with no arguments lists them as `[]`, not `{}`.
                    for name, argument in (endpoint.get("args") or {}).items():
                        kind = argument.get("type")
                        kind = "|".join(kind) if isinstance(kind, list) else kind
                        declared[name] = declared_text(kind, argument.get("description"))
                    return "the `wpcom/v2` route index", declared
        return None
    own = request["namespace"].removeprefix("/rest/v")
    for version in [own] + [v for v in HELP_VERSIONS if v != own]:
        for entry in helps[version]:
            if entry["method"] == request["method"] and shape(entry["path_labeled"]) == path:
                declared = {}
                for part in ("query", "body"):
                    for name, about in (entry["request"].get(part) or {}).items():
                        if isinstance(about, dict):
                            declared[name] = declared_text(about.get("type"), about.get("description"))
                        else:
                            declared[name] = declared_text(None, about)
                where = f"`rest/v{version}/help`"
                if version != own:
                    where += f", which documents the `rest/v{version}` endpoint only"
                return where, declared
    return None


def render(helps, index):
    out = [
        "# WordPress.com Reader endpoints",
        "",
        "Every WordPress.com Reader endpoint the library has: what it's for, the parameters the library "
        "sends, and what WordPress.com's public documentation declares for it.",
        "",
        "This file is written by `scripts/reader-endpoints-doc.py`. Don't edit it by hand: change the "
        "comment in the endpoint file or on the parameter's field, and run the script again. "
        "`Documentation/adding-a-wordpress-com-endpoint.md` says how an endpoint is added.",
        "",
        "## Where this comes from",
        "",
        "- **The library's own code.** Each endpoint's description is the comment on its request in "
        "`wp_api/src/wp_com/endpoint/reader_*_endpoint.rs`. Each parameter's is the comment on its field. "
        "They say what was seen when the endpoint was called, not what its code says.",
        "- **WordPress.com's public documentation of itself**: `https://public-api.wordpress.com/rest/v1.2/help` "
        "(and `v1.1`, `v1.3`) for the `rest/v1.x` endpoints, and `https://public-api.wordpress.com/wpcom/v2/` "
        "for the `wpcom/v2` routes. Neither needs a token. Neither lists every endpoint: one that isn't "
        "there says so below.",
        "",
        "Every `rest/v1.x` endpoint also declares `context`, `http_envelope`, `pretty`, `meta`, `fields` and "
        "`callback`. They're left out below unless the library sends them.",
        "",
        "## Contents",
        "",
    ]
    files = {path.name.removesuffix("_endpoint.rs"): path for path in ENDPOINTS.glob("reader_*_endpoint.rs")}
    for key, title in GROUPS.items():
        out.append(f"- [{title}](#{title.lower().replace(' ', '-')})")
    missing = set(files) - set(GROUPS)
    if missing:
        raise SystemExit(f"No title for the endpoint files: {sorted(missing)}")

    count = 0
    listed = {}
    for key, title in GROUPS.items():
        out += ["", f"## {title}", "", f"`wp_api/src/wp_com/endpoint/{key}_endpoint.rs`"]
        for request in parse_endpoint_file(files[key]):
            count += 1
            out += ["", f"### `{request['method']} {request['namespace']}{labeled(request['url'])}`", ""]
            out += [paragraph + "\n" if not paragraph.startswith("- ") else paragraph for paragraph in request["about"]]
            if request["about"] and request["about"][-1].startswith("- "):
                out.append("")
            names = f"Request `{request['enum']}::{request['variant']}`"
            if request["output"]:
                names += f", response `{request['output']}`"
            out += [names + ".", ""]

            sent = parse_params(request["params"]) if request["params"] else []
            where = "query" if request["method"] == "GET" else "body"
            heading = f"{request['method']} {request['namespace']}{labeled(request['url'])}"
            if sent and request["params"] in listed:
                names = ", ".join(f"`{field['name']}`" for field in sent)
                out += [
                    f"Parameters the library sends, in the {where} (`{request['params']}`): {names}. "
                    f"They're described under `{listed[request['params']]}`.",
                    "",
                ]
            elif sent:
                listed[request["params"]] = heading
                out.append(f"Parameters the library sends, in the {where} (`{request['params']}`):")
                out.append("")
                for field in sent:
                    optional = field["type"].startswith("Option<")
                    kind = field["type"].removeprefix("Option<").removesuffix(">") if optional else field["type"]
                    about = field["about"] or ["No comment on the field."]
                    # Paragraphs run on from the first. A list is nested, and a paragraph after it stands apart.
                    item, after_list = f"- `{field['name']}` (`{kind}`{'' if optional else ', required'}): {about[0]}", False
                    lines = []
                    for more in about[1:]:
                        if more.startswith("- "):
                            lines.append(f"  {more}")
                            after_list = True
                        elif after_list:
                            lines += ["", f"  {more}"]
                        else:
                            item += f" {more}"
                    out += [item] + lines
                out.append("")
            else:
                out += ["The library sends no parameters.", ""]

            declared = declared_for(request, helps, index)
            if declared is None:
                out.append("Public documentation: not listed.")
                continue
            source, arguments = declared
            names_sent = {field["name"] for field in sent}
            others = {name: about for name, about in arguments.items() if name not in names_sent and name not in COMMON}
            both = sorted(name for name in arguments if name in names_sent)
            undeclared = sorted(name for name in names_sent if name not in arguments)
            line = f"Public documentation: {source}."
            if both:
                line += " It declares " + ", ".join(f"`{name}`" for name in both) + ", which the library sends."
            if undeclared:
                line += " It doesn't declare " + ", ".join(f"`{name}`" for name in undeclared) + "."
            out.append(line)
            if others:
                out += ["", "Declared there, and not sent by the library:", ""]
                for name, about in others.items():
                    out.append(f"- `{name}`: {about}" if about else f"- `{name}`")

    out += [
        "",
        "## The page-view pixel",
        "",
        "### `GET https://pixel.wp.com/g.gif`",
        "",
        "Counts a view of a post in its site's stats. It isn't an API request: it goes to another host and "
        "carries no token. `wp_api/src/wp_com/reader_page_views.rs` builds it, and says which `User-Agent` a "
        "view is counted with.",
        "",
        "Parameters the library sends, in the query: `v=wpcom`, `reader=1`, `blog` (the site's ID), `post`, "
        "`host` (the site's host), `ref=https://wordpress.com/`, `t` (a number that differs each time), and "
        "`priv=1` for a private site.",
        "",
        "Public documentation: not listed.",
        "",
    ]
    return "\n".join(out), count


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--public-docs")
    options = parser.parse_args()
    helps, index = public_docs(options.public_docs)
    text, count = render(helps, index)
    text = re.sub(r"\n{3,}", "\n\n", text)
    if options.check:
        if DOCUMENT.read_text() != text:
            sys.exit(f"{DOCUMENT.relative_to(ROOT)} is out of date: run scripts/reader-endpoints-doc.py")
        print(f"{DOCUMENT.relative_to(ROOT)} is up to date ({count} endpoints)")
        return
    DOCUMENT.write_text(text)
    print(f"Wrote {DOCUMENT.relative_to(ROOT)}: {count} endpoints")


if __name__ == "__main__":
    main()
