//! Reading the HTTP `Link` header ([RFC 8288](https://www.rfc-editor.org/rfc/rfc8288)).
//!
//! WordPress uses it for the REST API root (`rel="https://api.w.org/"`) and for pagination
//! (`rel="next"`, `rel="prev"`):
//!
//! ```text
//! Link: <https://example.com/wp-json/>; rel="https://api.w.org/"
//! Link: <https://example.com/wp-json/wp/v2/posts?page=2>; rel="next"
//! ```

/// One entry of a `Link` header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkValue {
    /// The address between `<` and `>`, as written. It may be relative.
    pub target: String,
    /// The relation types in the entry's first `rel` parameter. Empty when it has none.
    pub rels: Vec<String>,
    /// The entry's first `anchor` parameter. A link with an anchor describes the resource the
    /// anchor names, not the response it came with (RFC 8288, section 3.2).
    pub anchor: Option<String>,
}

impl LinkValue {
    /// Whether the entry has the relation type `rel`. Relation types are compared ignoring
    /// ASCII case, as RFC 8288 says.
    pub fn has_rel(&self, rel: &str) -> bool {
        self.rels.iter().any(|r| r.eq_ignore_ascii_case(rel))
    }
}

/// Parses the value of one `Link` header.
///
/// An entry that isn't well formed is skipped, and the entries after it are still read. Commas
/// and semicolons inside `<…>` or a quoted parameter are part of it, not separators.
pub(crate) fn parse(header: &str) -> Vec<LinkValue> {
    let mut cursor = Cursor {
        bytes: header.as_bytes(),
        position: 0,
    };
    let mut links = Vec::new();
    while !cursor.is_at_end() {
        cursor.skip_whitespace_and(b',');
        if cursor.is_at_end() {
            break;
        }
        match parse_entry(&mut cursor, header) {
            Some(link) => links.push(link),
            // A target can't contain `<`, so one here starts the next entry. `parse_entry` has
            // already moved past this entry's own `<`, so this always makes progress.
            None if cursor.peek() == Some(b'<') => {}
            None => cursor.skip_to_next_entry(),
        }
    }
    links
}

fn parse_entry(cursor: &mut Cursor<'_>, header: &str) -> Option<LinkValue> {
    if !cursor.eat(b'<') {
        return None;
    }
    let start = cursor.position;
    cursor.advance_until(|b| matches!(b, b'>' | b'<'));
    let target = header[start..cursor.position].trim_ascii().to_string();
    if !cursor.eat(b'>') {
        return None;
    }

    let mut rels = None;
    let mut anchor = None;
    loop {
        cursor.skip_whitespace();
        match cursor.peek() {
            None | Some(b',') => break,
            Some(b';') => cursor.position += 1,
            Some(_) => return None,
        }
        cursor.skip_whitespace();
        let name_start = cursor.position;
        cursor.advance_until(|b| matches!(b, b'=' | b';' | b',') || b.is_ascii_whitespace());
        let name = &header[name_start..cursor.position];
        if name.is_empty() {
            // A stray `;`, as in `<a>;; rel=next`.
            continue;
        }
        cursor.skip_whitespace();
        let value = if cursor.eat(b'=') {
            cursor.skip_whitespace();
            cursor.parse_value(header)?
        } else {
            String::new()
        };
        // Only the first `rel` and `anchor` parameters count.
        if rels.is_none() && name.eq_ignore_ascii_case("rel") {
            rels = Some(value.split_ascii_whitespace().map(String::from).collect());
        } else if anchor.is_none() && name.eq_ignore_ascii_case("anchor") {
            anchor = Some(value);
        }
    }

    Some(LinkValue {
        target,
        rels: rels.unwrap_or_default(),
        anchor,
    })
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Cursor<'_> {
    fn is_at_end(&self) -> bool {
        self.position >= self.bytes.len()
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn eat(&mut self, byte: u8) -> bool {
        let matches = self.peek() == Some(byte);
        if matches {
            self.position += 1;
        }
        matches
    }

    fn advance_until(&mut self, stop: impl Fn(u8) -> bool) {
        while self.peek().is_some_and(|b| !stop(b)) {
            self.position += 1;
        }
    }

    fn skip_whitespace(&mut self) {
        self.advance_until(|b| !b.is_ascii_whitespace());
    }

    fn skip_whitespace_and(&mut self, separator: u8) {
        self.advance_until(|b| !b.is_ascii_whitespace() && b != separator);
    }

    /// Moves past the next comma that isn't inside a quoted string.
    ///
    /// This runs after an entry turned out not to be well formed, so it doesn't look for `<…>`:
    /// a stray `<` in the broken entry would otherwise swallow the next, good, entry. A comma
    /// inside a `<…>` in a broken entry only splits that entry into more broken pieces.
    fn skip_to_next_entry(&mut self) {
        let mut in_quotes = false;
        while let Some(byte) = self.peek() {
            self.position += 1;
            match byte {
                b'\\' if in_quotes => self.position += 1,
                b'"' => in_quotes = !in_quotes,
                b',' if !in_quotes => return,
                _ => {}
            }
        }
    }

    /// Reads a parameter value: a quoted string, with `\` escapes, or a token.
    fn parse_value(&mut self, header: &str) -> Option<String> {
        if !self.eat(b'"') {
            let start = self.position;
            self.advance_until(|b| matches!(b, b';' | b',') || b.is_ascii_whitespace());
            return Some(header[start..self.position].to_string());
        }
        let mut value = Vec::new();
        loop {
            match self.peek()? {
                b'"' => {
                    self.position += 1;
                    break;
                }
                b'\\' => {
                    self.position += 1;
                    value.push(self.peek()?);
                    self.position += 1;
                }
                byte => {
                    value.push(byte);
                    self.position += 1;
                }
            }
        }
        String::from_utf8(value).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn link(target: &str, rels: &[&str]) -> LinkValue {
        LinkValue {
            target: target.to_string(),
            rels: rels.iter().map(|r| r.to_string()).collect(),
            anchor: None,
        }
    }

    #[test]
    fn reads_what_wordpress_sends() {
        assert_eq!(
            parse(
                r#"<https://example.com/wp-json/wp/v2/posts?page=1>; rel="prev", <https://example.com/wp-json/wp/v2/posts?page=3>; rel="next""#
            ),
            [
                link("https://example.com/wp-json/wp/v2/posts?page=1", &["prev"]),
                link("https://example.com/wp-json/wp/v2/posts?page=3", &["next"]),
            ]
        );
        assert_eq!(
            parse(r#"<https://example.com/wp-json/>; rel="https://api.w.org/""#),
            [link(
                "https://example.com/wp-json/",
                &["https://api.w.org/"]
            )]
        );
    }

    #[rstest]
    #[case::root_relative("</wp-json/>; rel=a", "/wp-json/")]
    #[case::path_relative("<wp-json/>; rel=a", "wp-json/")]
    #[case::query_only("<?rest_route=/>; rel=a", "?rest_route=/")]
    #[case::comma_in_target(
        "<https://example.com/?ids=1,2>; rel=a",
        "https://example.com/?ids=1,2"
    )]
    #[case::semicolon_in_target(
        "<https://example.com/?a=1;b=2>; rel=a",
        "https://example.com/?a=1;b=2"
    )]
    #[case::whitespace_inside_brackets("< /wp-json/ >; rel=a", "/wp-json/")]
    fn reads_the_target_as_written(#[case] header: &str, #[case] target: &str) {
        assert_eq!(parse(header), [link(target, &["a"])]);
    }

    #[rstest]
    #[case::quoted(r#"<a>; rel="next""#, &["next"])]
    #[case::token("<a>; rel=next", &["next"])]
    #[case::several(r#"<a>; rel="next last""#, &["next", "last"])]
    #[case::uppercase_name(r#"<a>; REL="next""#, &["next"])]
    #[case::spaces_around_equals(r#"<a> ; rel = "next""#, &["next"])]
    #[case::other_params_first(r#"<a>; type="text/html"; title="a, b; c"; rel=next"#, &["next"])]
    #[case::escaped_quote(r#"<a>; title="say \"hi\""; rel=next"#, &["next"])]
    #[case::only_the_first_rel(r#"<a>; rel=next; rel=prev"#, &["next"])]
    #[case::no_rel("<a>; type=text/html", &[])]
    #[case::stray_semicolon("<a>;; rel=next", &["next"])]
    fn reads_the_rel_parameter(#[case] header: &str, #[case] rels: &[&str]) {
        assert_eq!(parse(header), [link("a", rels)]);
    }

    #[rstest]
    #[case::quoted(r##"<a>; rel=next; anchor="#comments""##, Some("#comments"))]
    #[case::token("<a>; anchor=/other; rel=next", Some("/other"))]
    #[case::uppercase_name("<a>; rel=next; ANCHOR=/other", Some("/other"))]
    #[case::only_the_first("<a>; anchor=/one; anchor=/two; rel=next", Some("/one"))]
    #[case::none("<a>; rel=next", None)]
    fn reads_the_anchor_parameter(#[case] header: &str, #[case] anchor: Option<&str>) {
        let links = parse(header);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].anchor.as_deref(), anchor);
        assert!(links[0].has_rel("next"));
    }

    // Headers from other APIs, taken from the test suites of `parse_link_header` and
    // `nom-rfc8288`.
    #[test]
    fn reads_github_pagination() {
        assert_eq!(
            parse(
                r#"<https://api.github.com/repositories/41986369/contributors?page=2>; rel="next", <https://api.github.com/repositories/41986369/contributors?page=14>; rel="last""#
            ),
            [
                link(
                    "https://api.github.com/repositories/41986369/contributors?page=2",
                    &["next"]
                ),
                link(
                    "https://api.github.com/repositories/41986369/contributors?page=14",
                    &["last"]
                ),
            ]
        );
    }

    #[test]
    fn reads_sentry_pagination() {
        // Parameters after `rel`, and a query with `&` first and `:` in a value.
        assert_eq!(
            parse(
                r#"<https://sentry.io/api/0/projects/1/groups/?&cursor=1420837590:0:1>; rel="previous"; results="false", <https://sentry.io/api/0/projects/1/groups/?&cursor=1420837533:0:0>; rel="next"; results="true""#
            ),
            [
                link(
                    "https://sentry.io/api/0/projects/1/groups/?&cursor=1420837590:0:1",
                    &["previous"]
                ),
                link(
                    "https://sentry.io/api/0/projects/1/groups/?&cursor=1420837533:0:0",
                    &["next"]
                ),
            ]
        );
    }

    #[rstest]
    #[case::escaped_quote(r#"</terms>; rel="copy\"right"; anchor=#foo"#, &["copy\"right"])]
    #[case::slash_in_quoted_value(r#"</terms>; rel="foo/bar""#, &["foo/bar"])]
    #[case::empty_quoted(r#"</terms>; rel="""#, &[])]
    #[case::quoted_comma_in_other_param(r#"</terms>; rel="origin"; csv="one,two""#, &["origin"])]
    fn reads_quoted_rel_values(#[case] header: &str, #[case] rels: &[&str]) {
        let links = parse(header);
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].target, "/terms");
        assert_eq!(links[0].rels, rels);
    }

    #[rstest]
    #[case::leading_commas(", , </a>; rel=next")]
    #[case::trailing_commas("</a>; rel=next , ,")]
    #[case::trailing_semicolon("</a>; rel=next;")]
    fn ignores_empty_list_entries(#[case] header: &str) {
        assert_eq!(parse(header), [link("/a", &["next"])]);
    }

    #[test]
    fn has_rel_ignores_case() {
        assert!(link("a", &["Next"]).has_rel("next"));
        assert!(!link("a", &["next"]).has_rel("prev"));
    }

    #[rstest]
    #[case::no_brackets(r#"https://example.com/; rel="a", </b>; rel="b""#)]
    #[case::junk_after_target(r#"</a> junk; rel="a", </b>; rel="b""#)]
    #[case::quoted_comma_in_skipped_entry(r#"</a> junk; title="x, y", </b>; rel="b""#)]
    #[case::stray_open_bracket(r#"junk <, </b>; rel="b""#)]
    #[case::bracketed_comma_in_skipped_entry(r#"junk <https://a/?i=1,2>; rel="a", </b>; rel="b""#)]
    #[case::unclosed_target(r#"<https://a/?x, </b>; rel="b""#)]
    #[case::open_bracket_after_target(r#"</a> <x, </b>; rel="b""#)]
    fn skips_an_entry_that_isnt_well_formed(#[case] header: &str) {
        assert_eq!(parse(header), [link("/b", &["b"])]);
    }

    #[test]
    fn an_unclosed_quote_ends_the_header() {
        // Everything after the quote is part of the quoted value, so nothing more can be read.
        assert_eq!(parse(r#"</a>; title="x, </b>; rel=b"#), []);
    }

    #[test]
    fn keeps_a_no_break_space_in_the_target() {
        // Only ASCII whitespace is trimmed, as `resolve_web_address` does.
        assert_eq!(
            parse("< /a\u{a0}>; rel=next"),
            [link("/a\u{a0}", &["next"])]
        );
    }

    #[rstest]
    #[case::empty("")]
    #[case::whitespace("   ")]
    #[case::commas(" , ,")]
    fn reads_nothing_from_an_empty_header(#[case] header: &str) {
        assert_eq!(parse(header), []);
    }
}
