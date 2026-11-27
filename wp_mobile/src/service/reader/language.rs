//! The reader's language, and the forms of it the Reader sends.
//!
//! What a language changes, measured on 2026-11-20 against the live API, signed out:
//!
//! | Request | Parameter | What it changes |
//! |---|---|---|
//! | A tag's posts, several tags' posts, Discover, the first posts of new sites | `_locale` | Which posts: all in that language |
//! | A search | `lang` | Which posts: all in that language |
//! | The menu | `locale` | The names of the Reader's own streams |
//! | The interests | `_locale` | Which tags are listed, and their names |
//!
//! Freshly Pressed, a site's or a feed's posts, a list's posts, related posts, Following, Liked
//! and a tag's info are the same in any language, so they're sent none.

use wp_api::wp_com::language::WPComLanguage;

/// The forms of the reader's language that are sent: one for the requests that pick posts, and one
/// for those that translate names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReaderLanguage {
    /// For the requests that pick posts in a language: a tag's posts, Discover, the first posts and
    /// a search. They read the language's first part only, and only in lower case, so `pt-br` and
    /// `pt`, or `zh-tw` and `zh-cn`, are the same posts. It's also the language's part of a stream's
    /// cache key.
    pub(crate) posts: &'static str,
    /// For the requests that translate names, the menu and the interests. They read a whole
    /// WordPress.com locale, such as `pt-br` or `zh-tw`.
    pub(crate) locale: &'static str,
}

impl ReaderLanguage {
    /// The forms of a WordPress.com language that the API answers in that language.
    ///
    /// - The posts are picked by WordPress.com's own code for the language, up to its first `-` or
    ///   `_`. That's what a search reads where it isn't the two-letter code: `bel` for Belarusian
    ///   and `kir` for Kyrgyz find posts, and `be` and `ky` find none.
    /// - The menu answers in English to most regional forms, such as `es-mx`, `de-ch` and
    ///   `de_formal`, so they're sent as the language they're a form of, `es` or `de`. That also
    ///   suits the interests, which read those forms as that language anyway. Serbian in Latin
    ///   script is kept: the menu answers it in Latin script, and in Cyrillic to `sr`.
    pub(crate) fn new(language: WPComLanguage) -> Self {
        let slug = language.slug_str();
        let posts = slug.split(['-', '_']).next().unwrap_or(slug);
        let locale = match language {
            WPComLanguage::SerbianLatin => slug,
            _ => language.parent_locale_slug_str().unwrap_or(slug),
        };
        Self { posts, locale }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(WPComLanguage::English, "en", "en")]
    #[case(WPComLanguage::French, "fr", "fr")]
    #[case(WPComLanguage::FrenchCanada, "fr", "fr")]
    #[case(WPComLanguage::Portuguese, "pt", "pt")]
    #[case(WPComLanguage::PortugueseBrazil, "pt", "pt-br")]
    #[case(WPComLanguage::ChineseSimplified, "zh", "zh-cn")]
    #[case(WPComLanguage::ChineseTraditional, "zh", "zh-tw")]
    #[case(WPComLanguage::ChineseHongKong, "zh", "zh-hk")]
    #[case(WPComLanguage::ChineseSingapore, "zh", "zh-cn")]
    #[case(WPComLanguage::SpanishMexico, "es", "es")]
    #[case(WPComLanguage::GermanSwitzerland, "de", "de")]
    #[case(WPComLanguage::GermanFormal, "de", "de")]
    #[case(WPComLanguage::SerbianLatin, "sr", "sr_latin")]
    #[case(WPComLanguage::GreekPolytonic, "el", "el-po")]
    #[case(WPComLanguage::EnglishUK, "en", "en-gb")]
    #[case(WPComLanguage::Belarusian, "bel", "bel")]
    #[case(WPComLanguage::Kyrgyz, "kir", "kir")]
    #[case(WPComLanguage::Japanese, "ja", "ja")]
    fn test_the_forms_of_a_language_that_are_sent(
        #[case] language: WPComLanguage,
        #[case] posts: &str,
        #[case] locale: &str,
    ) {
        let sent = ReaderLanguage::new(language);
        assert_eq!((sent.posts, sent.locale), (posts, locale));
    }

    /// Every form sent is lower case, as the requests that pick posts need: they answer none to
    /// `FR`.
    #[test]
    fn test_every_language_is_sent_in_lower_case() {
        for language in WPComLanguage::all() {
            let sent = ReaderLanguage::new(language);
            for form in [sent.posts, sent.locale] {
                assert!(!form.is_empty(), "{language:?}");
                assert_eq!(form, form.to_ascii_lowercase(), "{language:?}");
            }
        }
    }
}
