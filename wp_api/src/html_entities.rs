//! Decoding the HTML character references that API text fields carry.
//!
//! Many WordPress fields are HTML, not plain text, even when they hold nothing
//! but a short line of text. A post title, for example, goes through
//! `wptexturize()` on the server, so `Tom & Jerry - the movie` arrives as
//! `Tom &#038; Jerry &#8211; the movie`. A client that shows the field as text
//! has to decode it first.
//!
//! Decoding follows the WHATWG HTML specification for text content, so the
//! result matches what a browser shows, including for the malformed references
//! that older content still carries.

/// Decodes the HTML character references in `text`, the way a browser decodes
/// text content.
///
/// Use this to show a field the API returns as HTML, such as a post title, as
/// plain text. It decodes named references (`&amp;`), decimal references
/// (`&#8211;`) and hexadecimal references (`&#x2013;`), following the WHATWG
/// HTML specification:
///
/// - The legacy named references that browsers accept without a semicolon,
///   such as `&copy`, are decoded. Other names need the semicolon.
/// - Numeric references in the range 0x80–0x9F are read as Windows-1252, so
///   `&#150;` is an en dash.
/// - A numeric reference to zero, a surrogate, or a value past U+10FFFF
///   becomes U+FFFD REPLACEMENT CHARACTER.
/// - Anything that is not a reference, such as a bare `&` or an unknown name,
///   is left as it is. A legacy name is still decoded when other text follows
///   it, so an unescaped query string like `?a=1&region=us` becomes
///   `?a=1®ion=us`, as it does in a browser. WordPress escapes that `&` in
///   the fields it texturizes.
///
/// It decodes once: `&amp;amp;` becomes `&amp;`. It does not remove or
/// interpret tags, so `<em>` stays in the text, while `&lt;em&gt;` becomes
/// `<em>`.
#[uniffi::export]
pub fn decode_html_entities(text: String) -> String {
    match htmlize::unescape(text.as_str()) {
        std::borrow::Cow::Borrowed(_) => text,
        std::borrow::Cow::Owned(decoded) => decoded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn decode(text: &str) -> String {
        decode_html_entities(text.to_string())
    }

    #[test]
    fn decodes_a_saved_post_title_from_the_live_api() {
        assert_eq!(
            decode("WooCommerce for Android 3.5 is rolled out &#8211; 3.6 in code freeze"),
            "WooCommerce for Android 3.5 is rolled out – 3.6 in code freeze"
        );
    }

    // What `wptexturize()` and `esc_html()` produce, which is most of what the
    // API sends.
    #[rstest]
    #[case::en_dash("&#8211;", "–")]
    #[case::em_dash("&#8212;", "—")]
    #[case::left_single_quote("&#8216;", "‘")]
    #[case::right_single_quote("&#8217;", "’")]
    #[case::left_double_quote("&#8220;", "“")]
    #[case::right_double_quote("&#8221;", "”")]
    #[case::ellipsis("&#8230;", "…")]
    #[case::multiplication_sign("&#215;", "×")]
    #[case::prime("&#8242;", "′")]
    #[case::double_prime("&#8243;", "″")]
    #[case::texturized_ampersand("&#038;", "&")]
    #[case::escaped_apostrophe("&#039;", "'")]
    #[case::apostrophe_without_padding("&#39;", "'")]
    #[case::escaped_quote("&quot;", "\"")]
    #[case::escaped_less_than("&lt;", "<")]
    #[case::escaped_greater_than("&gt;", ">")]
    #[case::escaped_ampersand("&amp;", "&")]
    #[case::non_breaking_space("&nbsp;", "\u{a0}")]
    fn decodes_what_wordpress_emits(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(decode(input), expected);
    }

    #[rstest]
    #[case::decimal("&#8211;", "–")]
    #[case::decimal_with_leading_zeros("&#0000038;", "&")]
    #[case::lowercase_hex("&#x2013;", "–")]
    #[case::uppercase_hex_marker("&#X2013;", "–")]
    #[case::uppercase_hex_digits("&#x1F600;", "😀")]
    #[case::mixed_case_hex_digits("&#x1f60A;", "😊")]
    #[case::hex_with_leading_zeros("&#x0000041;", "A")]
    #[case::ascii("&#65;&#66;&#67;", "ABC")]
    #[case::astral_plane("&#128512;", "😀")]
    #[case::highest_code_point("&#x10FFFF;", "\u{10FFFF}")]
    fn decodes_numeric_references(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(decode(input), expected);
    }

    #[rstest]
    #[case::copyright("&copy;", "©")]
    #[case::registered("&reg;", "®")]
    #[case::trademark("&trade;", "™")]
    #[case::euro("&euro;", "€")]
    #[case::accented("&eacute;&Eacute;", "éÉ")]
    #[case::greek("&alpha;&Omega;", "αΩ")]
    #[case::arrow("&rarr;", "→")]
    #[case::uppercase_alias("&AMP;", "&")]
    #[case::two_code_points("&acE;", "∾\u{333}")]
    #[case::long_name("&NotNestedGreaterGreater;", "⪢\u{338}")]
    #[case::html5_only("&nbsp;&ThinSpace;", "\u{a0}\u{2009}")]
    #[case::fractions("&frac12;&frac14;&frac34;", "½¼¾")]
    #[case::superscripts("&sup1;&sup2;&sup3;", "¹²³")]
    #[case::soft_hyphen("co&shy;op", "co\u{ad}op")]
    // HTML 4 and PHP map these to U+2329 and U+232A.
    #[case::angle_brackets("&lang;x&rang;", "⟨x⟩")]
    fn decodes_named_references(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(decode(input), expected);
    }

    // Browsers accept a short list of legacy names without the semicolon.
    #[rstest]
    #[case::copyright("&copy 2024", "© 2024")]
    #[case::ampersand("&amp more", "& more")]
    #[case::less_than("&lt;b&gt", "<b>")]
    #[case::non_breaking_space("a&nbspb", "a\u{a0}b")]
    #[case::at_the_end("&copy", "©")]
    #[case::name_with_digits("1&frac12 cups", "1½ cups")]
    #[case::name_ending_in_a_digit("x&sup2", "x²")]
    #[case::numeric_decimal("&#8211 next", "– next")]
    #[case::numeric_hex("&#x2013 next", "– next")]
    fn decodes_references_without_a_semicolon(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(decode(input), expected);
    }

    #[rstest]
    #[case::modern_name("&hellip next", "&hellip next")]
    #[case::modern_name_at_the_end("&rarr", "&rarr")]
    fn leaves_modern_names_without_a_semicolon(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(decode(input), expected);
    }

    // The longest legacy name that matches is used, the way browsers do it.
    #[rstest]
    #[case::full_name("&notin;", "∉")]
    #[case::legacy_prefix("&notit;", "¬it;")]
    #[case::legacy_prefix_without_semicolon("&notit", "¬it")]
    fn matches_the_longest_name(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(decode(input), expected);
    }

    // Older content stored with a Windows-1252 charset carries these.
    #[rstest]
    #[case(0x80, '€')]
    #[case(0x82, '‚')]
    #[case(0x83, 'ƒ')]
    #[case(0x84, '„')]
    #[case(0x85, '…')]
    #[case(0x86, '†')]
    #[case(0x87, '‡')]
    #[case(0x88, 'ˆ')]
    #[case(0x89, '‰')]
    #[case(0x8A, 'Š')]
    #[case(0x8B, '‹')]
    #[case(0x8C, 'Œ')]
    #[case(0x8E, 'Ž')]
    #[case(0x91, '‘')]
    #[case(0x92, '’')]
    #[case(0x93, '“')]
    #[case(0x94, '”')]
    #[case(0x95, '•')]
    #[case(0x96, '–')]
    #[case(0x97, '—')]
    #[case(0x98, '˜')]
    #[case(0x99, '™')]
    #[case(0x9A, 'š')]
    #[case(0x9B, '›')]
    #[case(0x9C, 'œ')]
    #[case(0x9E, 'ž')]
    #[case(0x9F, 'Ÿ')]
    fn reads_the_c1_range_as_windows_1252(#[case] code: u32, #[case] expected: char) {
        assert_eq!(decode(&format!("&#{code};")), expected.to_string());
        assert_eq!(decode(&format!("&#x{code:X};")), expected.to_string());
    }

    // Windows-1252 leaves these five undefined, so they stay as C1 controls.
    #[rstest]
    #[case(0x81)]
    #[case(0x8D)]
    #[case(0x8F)]
    #[case(0x90)]
    #[case(0x9D)]
    fn keeps_the_undefined_windows_1252_codes(#[case] code: u32) {
        let expected = char::from_u32(code).unwrap().to_string();
        assert_eq!(decode(&format!("&#{code};")), expected);
    }

    #[rstest]
    #[case::null("&#0;")]
    #[case::null_hex("&#x0;")]
    #[case::lowest_surrogate("&#xD800;")]
    #[case::highest_surrogate("&#xDFFF;")]
    #[case::surrogate_decimal("&#55296;")]
    #[case::past_the_last_code_point("&#x110000;")]
    #[case::far_past_the_last_code_point("&#xFFFFFFFF;")]
    #[case::overflows_u32("&#99999999999999999999;")]
    #[case::overflows_u64("&#x10000000000000000000000;")]
    fn replaces_invalid_code_points(#[case] input: &str) {
        assert_eq!(decode(input), "\u{FFFD}");
    }

    // The spec calls these parse errors but still emits the code point.
    #[rstest]
    #[case::carriage_return("&#13;", "\r")]
    #[case::padded_carriage_return("&#013;", "\r")]
    #[case::line_feed("&#10;", "\n")]
    #[case::padded_line_feed("&#010;", "\n")]
    #[case::padded_tab("&#009;", "\t")]
    #[case::tab("&#9;", "\t")]
    #[case::delete("&#x7F;", "\u{7F}")]
    #[case::noncharacter("&#xFFFF;", "\u{FFFF}")]
    #[case::noncharacter_in_an_astral_plane("&#x1FFFE;", "\u{1FFFE}")]
    fn keeps_controls_and_noncharacters(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(decode(input), expected);
    }

    #[rstest]
    #[case::empty("")]
    #[case::plain_text("Hello, world")]
    #[case::bare_ampersand("AT&T and R&D")]
    #[case::trailing_ampersand("Tom &")]
    #[case::ampersand_and_space("Tom & Jerry")]
    #[case::unknown_name("&unknown;")]
    #[case::unknown_name_without_semicolon("&unknown")]
    #[case::wrong_case("&Amp;")]
    #[case::hash_only("&#")]
    #[case::hash_and_semicolon("&#;")]
    #[case::hex_marker_only("&#x")]
    #[case::hex_marker_and_semicolon("&#x;")]
    #[case::not_hex_digits("&#xZZ;")]
    #[case::not_decimal_digits("&#abc;")]
    #[case::space_inside("& amp;")]
    #[case::tags("<em>Hello</em> <a href=\"x\">there</a>")]
    #[case::non_ascii("Café – 日本語 😀")]
    fn leaves_text_that_is_not_a_reference(#[case] input: &str) {
        assert_eq!(decode(input), input);
    }

    #[test]
    fn decodes_only_once() {
        assert_eq!(decode("&amp;amp;"), "&amp;");
        assert_eq!(decode("&amp;#8211;"), "&#8211;");
        assert_eq!(decode("&amp;lt;b&amp;gt;"), "&lt;b&gt;");
    }

    #[test]
    fn decodes_references_next_to_each_other() {
        assert_eq!(decode("&lt;&gt;&amp;&quot;&#39;"), "<>&\"'");
        assert_eq!(decode("&#8220;&#8221;"), "“”");
    }

    #[test]
    fn decodes_references_at_either_end_of_the_text() {
        assert_eq!(decode("&#8220;Quoted&#8221;"), "“Quoted”");
    }

    #[test]
    fn keeps_the_text_around_references() {
        assert_eq!(
            decode("日本語 &#8211; Café &amp; 😀 &hellip; end"),
            "日本語 – Café & 😀 … end"
        );
    }

    #[test]
    fn decodes_escaped_tags_without_touching_real_ones() {
        assert_eq!(
            decode("<em>Use &lt;br&gt; here</em>"),
            "<em>Use <br> here</em>"
        );
    }

    #[test]
    fn decodes_long_text() {
        let input = "Tom &#038; Jerry &#8211; ".repeat(10_000);
        let expected = "Tom & Jerry – ".repeat(10_000);
        assert_eq!(decode(&input), expected);
    }

    #[test]
    fn decodes_a_long_run_of_ampersands() {
        let input = "&".repeat(10_000);
        assert_eq!(decode(&input), input);
    }

    #[test]
    fn decodes_a_very_long_numeric_reference() {
        let input = format!("&#{};", "0".repeat(10_000) + "65");
        assert_eq!(decode(&input), "A");
    }

    // Escaping and then decoding gives back the original text, for text that
    // is awkward to escape.
    #[rstest]
    #[case("")]
    #[case("plain")]
    #[case("Tom & Jerry")]
    #[case("&amp; is already escaped")]
    #[case("<script>alert(\"x\")</script>")]
    #[case("a < b && c > d")]
    #[case("&#8211; looks like a reference")]
    #[case("Café – 日本語 😀")]
    #[case("&&&;;;###")]
    fn reverses_escaping(#[case] text: &str) {
        assert_eq!(decode(&htmlize::escape_text(text)), text);
        assert_eq!(decode(&htmlize::escape_all_quotes(text)), text);
    }

    // Locales translate the quotes and primes `wptexturize()` uses.
    #[rstest]
    #[case::german_quotes("&#8222;Zitat&#8220;", "„Zitat“")]
    #[case::german_single_quotes("&#8218;Zitat&#8216;", "‚Zitat‘")]
    #[case::guillemets("&#171;Цитата&#187;", "«Цитата»")]
    #[case::french_primes("9&prime; 6&Prime;", "9′ 6″")]
    #[case::french_apostrophe("l&rsquo;&lsquo;", "l’‘")]
    #[case::norwegian_quotes("&laquo;sitat&raquo;", "«sitat»")]
    fn decodes_translated_wptexturize_output(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(decode(input), expected);
    }

    // Forms identified in a test corpus of post titles and excerpts, beyond
    // the `wptexturize()` and `esc_html()` output above. Authors and
    // importers type references by hand, `wp_encode_emoji()` writes emoji as
    // lowercase hex, and older content keeps zero padding and Windows-1252
    // codes.
    #[rstest]
    #[case::padded_plus("X-post from &#043;team", "X-post from +team")]
    #[case::padded_hash("X-post from &#035;tag", "X-post from #tag")]
    #[case::padded_quote("&#034;Quoted&#034;", "\"Quoted\"")]
    #[case::unpadded_ampersand("Q&#38;A", "Q&A")]
    #[case::texturized_url(
        "https://example.com/?p=1&#038;region=us",
        "https://example.com/?p=1&region=us"
    )]
    #[case::escaped_url("?p=1&amp;section=2", "?p=1&section=2")]
    #[case::unpadded_quote("&#34;", "\"")]
    #[case::unpadded_plus("&#43;", "+")]
    #[case::at_sign("me&#64;example.com", "me@example.com")]
    #[case::vertical_bar("a &#124; b", "a | b")]
    #[case::hex_apostrophe("I&#x27;m", "I'm")]
    #[case::hex_right_single_quote("Here&#x2019;s", "Here’s")]
    #[case::hex_non_breaking_space("end.&#xA0; Next", "end.\u{a0} Next")]
    #[case::decimal_non_breaking_space("&#160;", "\u{a0}")]
    #[case::trademark_decimal("&#8482;", "™")]
    #[case::bullet("&#8226;", "•")]
    #[case::footnote_back_arrow("&#8617;", "↩")]
    #[case::heart("&#10084;", "❤")]
    #[case::check_box("&#9745;", "☑")]
    #[case::male_sign("&#9794;", "♂")]
    #[case::stopwatch("&#9202;", "⏲")]
    #[case::one_half("&#189;", "½")]
    #[case::pound_decimal("&#163;", "£")]
    #[case::e_acute_decimal("&#233;", "é")]
    #[case::u_umlaut_decimal("&#252;", "ü")]
    #[case::windows_1252_apostrophe("don&#146;t", "don’t")]
    #[case::emoji_decimal("&#128525;", "😍")]
    #[case::emoji_lowercase_hex("&#x1f631;", "😱")]
    #[case::emoji_with_skin_tone("&#x1f44d;&#x1f3fc;", "👍🏼")]
    #[case::emoji_with_variation_selector("&#x2764;&#xfe0f;", "❤️")]
    #[case::emoji_with_zero_width_joiner("&#128105;&#8205;&#128187;", "👩\u{200d}💻")]
    #[case::flag("&#127462;&#127482;", "🇦🇺")]
    #[case::repeated_emoji("&#x1f355;&#x1f355;&#x1f355;", "🍕🍕🍕")]
    #[case::hellip("Wait&hellip;", "Wait…")]
    #[case::mdash("&mdash;", "—")]
    #[case::ndash("&ndash;", "–")]
    #[case::rsquo("it&rsquo;s", "it’s")]
    #[case::lsquo("&lsquo;", "‘")]
    #[case::ldquo_rdquo("&ldquo;Hi&rdquo;", "“Hi”")]
    #[case::raquo("Read more &raquo;", "Read more »")]
    #[case::apos("&apos;", "'")]
    #[case::pound("&pound;5", "£5")]
    #[case::hearts("&hearts;", "♥")]
    #[case::thin_space("&thinsp;", "\u{2009}")]
    #[case::flat("B&flat;", "B♭")]
    #[case::acute_accent("&acute;", "´")]
    #[case::accented_letters("&uuml;&ouml;&ccedil;&Ccedil;&oslash;", "üöçÇø")]
    fn decodes_references_seen_in_a_test_corpus(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(decode(input), expected);
    }

    // An unescaped query string is left alone unless a parameter starts with
    // a legacy name, which a browser decodes too.
    #[rstest]
    #[case::not_a_legacy_name("?p=123&preview=true", "?p=123&preview=true")]
    #[case::region("?lang=en&region=us", "?lang=en®ion=us")]
    #[case::section("?a=1&section=2", "?a=1§ion=2")]
    #[case::copy("?a=1&copy=2", "?a=1©=2")]
    #[case::notify("?a=1&notify=1", "?a=1¬ify=1")]
    fn decodes_legacy_names_in_query_strings(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(decode(input), expected);
    }
}
