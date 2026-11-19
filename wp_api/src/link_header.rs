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
    cursor.advance_until(|b| b == b'>');
    let target = header[start..cursor.position].trim().to_string();
    if !cursor.eat(b'>') {
        return None;
    }

    let mut rels = None;
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
        // Only the first `rel` parameter counts.
        if rels.is_none() && name.eq_ignore_ascii_case("rel") {
            rels = Some(value.split_ascii_whitespace().map(String::from).collect());
        }
    }

    Some(LinkValue {
        target,
        rels: rels.unwrap_or_default(),
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

    /// Moves past the next comma that isn't inside `<…>` or a quoted string.
    fn skip_to_next_entry(&mut self) {
        let mut in_target = false;
        let mut in_quotes = false;
        while let Some(byte) = self.peek() {
            self.position += 1;
            match byte {
                b'\\' if in_quotes => self.position += 1,
                b'"' if !in_target => in_quotes = !in_quotes,
                b'<' if !in_quotes => in_target = true,
                b'>' if !in_quotes => in_target = false,
                b',' if !in_target && !in_quotes => return,
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

    #[test]
    fn has_rel_ignores_case() {
        assert!(link("a", &["Next"]).has_rel("next"));
        assert!(!link("a", &["next"]).has_rel("prev"));
    }

    #[rstest]
    #[case::no_brackets(r#"https://example.com/; rel="a", </b>; rel="b""#)]
    #[case::junk_after_target(r#"</a> junk; rel="a", </b>; rel="b""#)]
    #[case::quoted_comma_in_skipped_entry(r#"</a> junk; title="x, y", </b>; rel="b""#)]
    fn skips_an_entry_that_isnt_well_formed(#[case] header: &str) {
        assert_eq!(parse(header), [link("/b", &["b"])]);
    }

    #[test]
    fn an_unclosed_quote_ends_the_header() {
        // Everything after the quote is part of the quoted value, so nothing more can be read.
        assert_eq!(parse(r#"</a>; title="x, </b>; rel=b"#), []);
    }

    #[rstest]
    #[case::empty("")]
    #[case::whitespace("   ")]
    #[case::commas(" , ,")]
    fn reads_nothing_from_an_empty_header(#[case] header: &str) {
        assert_eq!(parse(header), []);
    }
}
