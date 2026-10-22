/// The stylesheet for the Reader's post page, for a `<style>` in the page's `<head>`.
///
/// It takes its colours from the variables [`reader_page_colors_css`](super::reader_page_colors_css)
/// sets, so put that CSS on the page too.
#[uniffi::export]
pub fn reader_stylesheet() -> String {
    STYLESHEET.to_owned()
}

/// The script for the Reader's post page, for a `<script>` in the page's `<head>`.
///
/// It fades images in as they load, makes images open in the app's image viewer, and sends the app
/// messages through a handler named `readerPost`. The script's own comments list the messages.
#[uniffi::export]
pub fn reader_script() -> String {
    SCRIPT.to_owned()
}

const STYLESHEET: &str = include_str!("reader.css");
const SCRIPT: &str = include_str!("reader.js");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::{
        ReaderFixedTheme, ReaderFont, ReaderPageColors, reader_page_colors_css,
        reader_page_typography_css,
    };
    use std::collections::BTreeSet;

    /// The stylesheet without its comments, which can mention anything.
    fn stylesheet_rules() -> String {
        let mut rules = String::new();
        let mut rest = STYLESHEET;
        while let Some((before, after)) = rest.split_once("/*") {
            rules.push_str(before);
            rest = after.split_once("*/").expect("comments are closed").1;
        }
        rules.push_str(rest);
        rules
    }

    fn variable_names(css: &str, prefix: &str) -> BTreeSet<String> {
        css.split(prefix)
            .skip(1)
            .map(|rest| {
                let end = rest
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                    .unwrap_or(rest.len());
                rest[..end].to_owned()
            })
            .collect()
    }

    #[test]
    fn reads_only_the_variables_the_page_sets() {
        let sepia = ReaderPageColors::Fixed {
            theme: ReaderFixedTheme::Sepia,
        };
        let page = reader_page_colors_css(sepia);
        let typography = reader_page_typography_css(ReaderFont::Serif, 1.0, sepia);
        // The stylesheet sets some of its own, like the space between blocks.
        let set = variable_names(&(page + &typography + &stylesheet_rules()), "\n  --");
        let read = variable_names(&stylesheet_rules(), "var(--");

        let unset: Vec<_> = read.difference(&set).collect();
        assert!(unset.is_empty(), "nothing sets {unset:?}");
    }

    #[test]
    fn ignores_the_system_appearance() {
        assert!(!stylesheet_rules().contains("prefers-color-scheme"));
    }

    #[test]
    fn takes_its_colors_from_the_theme() {
        // Every rule, as its selector and declarations.
        let rules = stylesheet_rules();
        let fixed_colors: Vec<_> = rules
            .split('}')
            .filter_map(|rule| rule.rsplit_once('{'))
            .filter(|(_, declarations)| {
                ["#", "rgb(", "rgba(", "hsl(", "hsla("]
                    .iter()
                    .any(|color| declarations.contains(color))
            })
            .map(|(selector, _)| selector.trim())
            .collect();

        // Gallery captions sit on the image, not the page. The author's colours only apply to
        // something that sets both its text and background colour, so it doesn't show on the page.
        // A gist's code takes GitHub's syntax colours, light or dark to match the theme.
        let off_image: Vec<_> = fixed_colors
            .iter()
            .filter(|selector| !selector.ends_with("figcaption"))
            .filter(|selector| !selector.starts_with(".has-background.has-text-color.has-"))
            .filter(|selector| !(selector.starts_with("body .gist ") && selector.contains(".pl-")))
            .collect();
        assert!(off_image.is_empty(), "fixed colours in {off_image:#?}");
    }

    #[test]
    fn styles_the_formatting_wordpress_adds() {
        let rules = stylesheet_rules();
        let classes = [
            "has-drop-cap",
            "has-text-align-left",
            "has-text-align-center",
            "has-text-align-right",
            "has-small-font-size",
            "has-medium-font-size",
            "has-large-font-size",
            "has-x-large-font-size",
        ];
        let unstyled: Vec<_> = classes
            .iter()
            .filter(|class| !rules.contains(&format!(".{class}")))
            .collect();
        assert!(unstyled.is_empty(), "unstyled: {unstyled:?}");

        // Every colour in WordPress's palette, as text and as a background.
        let palette = [
            "black",
            "cyan-bluish-gray",
            "white",
            "pale-pink",
            "vivid-red",
            "luminous-vivid-orange",
            "luminous-vivid-amber",
            "light-green-cyan",
            "vivid-green-cyan",
            "pale-cyan-blue",
            "vivid-cyan-blue",
            "vivid-purple",
        ];
        for color in palette {
            for class in [
                format!("has-{color}-color"),
                format!("has-{color}-background-color"),
            ] {
                assert!(
                    rules.contains(&format!(".has-background.has-text-color.{class} {{")),
                    "{class} isn't styled"
                );
            }
        }
    }

    #[test]
    fn styles_the_classes_the_script_adds() {
        let added = variable_names(SCRIPT, "classList.add(\"");
        assert!(!added.is_empty());

        let rules = stylesheet_rules();
        let unstyled: Vec<_> = added
            .iter()
            .filter(|class| !rules.contains(&format!(".{class}")))
            .collect();
        assert!(unstyled.is_empty(), "nothing styles {unstyled:?}");
    }
}
