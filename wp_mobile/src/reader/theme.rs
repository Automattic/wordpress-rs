/// An sRGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Record)]
pub struct ReaderColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    /// Opacity, from 0 (transparent) to 255 (opaque).
    pub alpha: u8,
}

impl ReaderColor {
    const fn rgb(hex: u32) -> Self {
        Self {
            red: (hex >> 16) as u8,
            green: (hex >> 8) as u8,
            blue: hex as u8,
            alpha: u8::MAX,
        }
    }

    fn with_opacity(self, opacity: f32) -> Self {
        Self {
            alpha: (opacity * f32::from(u8::MAX)).round() as u8,
            ..self
        }
    }

    /// The colour as a CSS hex colour: `#rrggbb`, or `#rrggbbaa` if it isn't opaque.
    fn css(self) -> String {
        let Self {
            red,
            green,
            blue,
            alpha,
        } = self;
        if alpha == u8::MAX {
            format!("#{red:02x}{green:02x}{blue:02x}")
        } else {
            format!("#{red:02x}{green:02x}{blue:02x}{alpha:02x}")
        }
    }
}

/// The colours a Reader theme draws the post page with.
///
/// Each one is a CSS variable on the page, named after its field: `text` is `--reader-text`, and
/// so on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Record)]
pub struct ReaderThemeColors {
    /// Body text and headings.
    pub text: ReaderColor,
    /// Less prominent text, like captions and quote citations.
    pub secondary_text: ReaderColor,
    /// The least prominent text, like placeholders.
    pub tertiary_text: ReaderColor,
    /// The page background.
    pub background: ReaderColor,
    /// Backgrounds that set content apart from the page, like code blocks.
    pub secondary_background: ReaderColor,
    /// Borders and separators, like table borders and the rule beside a quote.
    pub border: ReaderColor,
    /// Links, and the social icons, which are links too.
    ///
    /// In the colours an app supplies for [`ReaderPageColors::System`], this is the app's brand
    /// colour: green for Jetpack, blue for WordPress. Pick shades that read as text on the
    /// background, at a contrast of 4.5:1 or more. The themes with fixed colours use their text
    /// colour for links instead, whatever the app.
    pub link: ReaderColor,
}

/// The Reader themes with fixed colours.
///
/// The default theme isn't here: it uses the platform's own colours, which only the app knows, so
/// the app supplies them in [`ReaderPageColors::System`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum ReaderFixedTheme {
    Soft,
    Sepia,
    Evening,
    Oled,
    Hacker,
    Candy,
}

/// The colours for a theme with fixed colours.
///
/// Every colour comes from the theme's text colour, except the background.
#[uniffi::export]
pub fn reader_fixed_theme_colors(theme: ReaderFixedTheme) -> ReaderThemeColors {
    let (text, background) = match theme {
        ReaderFixedTheme::Soft => (0x2d2e2e, 0xf2f2f2),
        ReaderFixedTheme::Sepia => (0x27201b, 0xeae0cd),
        ReaderFixedTheme::Evening => (0xabaab2, 0x3a3a3c),
        ReaderFixedTheme::Oled => (0xffffff, 0x000000),
        ReaderFixedTheme::Hacker => (0x00ff00, 0x000000),
        ReaderFixedTheme::Candy => (0x0066ff, 0xffe8fd),
    };
    let text = ReaderColor::rgb(text);
    // Faint colours need a little more opacity to show up on a dark background.
    let is_dark = is_dark(theme);

    ReaderThemeColors {
        text,
        secondary_text: text.with_opacity(0.6),
        tertiary_text: text.with_opacity(if is_dark { 0.45 } else { 0.33 }),
        background: ReaderColor::rgb(background),
        secondary_background: text.with_opacity(if is_dark { 0.15 } else { 0.1 }),
        border: text.with_opacity(0.3),
        link: text,
    }
}

fn is_dark(theme: ReaderFixedTheme) -> bool {
    match theme {
        ReaderFixedTheme::Evening | ReaderFixedTheme::Oled | ReaderFixedTheme::Hacker => true,
        ReaderFixedTheme::Soft | ReaderFixedTheme::Sepia | ReaderFixedTheme::Candy => false,
    }
}

/// The colours to draw the Reader's post page with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum ReaderPageColors {
    /// The default theme: the platform's colours, which follow the system's light or dark
    /// appearance, with the app's brand colour for links.
    System {
        light: ReaderThemeColors,
        dark: ReaderThemeColors,
    },
    /// A theme with fixed colours, whatever the system appearance.
    Fixed { theme: ReaderFixedTheme },
}

/// Whether a theme's colours follow the system's light or dark appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum ReaderThemeAppearance {
    /// The theme has light and dark colours, and follows the system's appearance. The app can
    /// force either by setting its web view's appearance, and the page follows.
    FollowsSystem,
    /// The theme's colours are light, whatever the system's appearance.
    AlwaysLight,
    /// The theme's colours are dark, whatever the system's appearance.
    AlwaysDark,
}

#[uniffi::export]
impl ReaderPageColors {
    /// Whether the theme follows the system's light or dark appearance, or always looks the same.
    ///
    /// A theme that always looks the same ignores a forced appearance. Whether it's light or dark
    /// tells the app how to draw its own controls around the post.
    pub fn appearance(&self) -> ReaderThemeAppearance {
        match self {
            ReaderPageColors::System { .. } => ReaderThemeAppearance::FollowsSystem,
            ReaderPageColors::Fixed { theme } if is_dark(*theme) => {
                ReaderThemeAppearance::AlwaysDark
            }
            ReaderPageColors::Fixed { .. } => ReaderThemeAppearance::AlwaysLight,
        }
    }
}

/// The typeface for the post's text, from the reader's settings.
///
/// A theme that follows the system only uses the platform's own fonts. The themes with fixed
/// colours can use fonts the apps bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum ReaderFont {
    /// The platform's own: San Francisco on Apple's platforms, Roboto on Android.
    Sans,
    /// The platform's serif, in a theme that follows the system: New York on Apple's platforms,
    /// Noto Serif on Android. In the other themes, Noto Serif, which the iOS app bundles.
    Serif,
    /// The platform's monospaced typeface.
    Mono,
}

impl ReaderFont {
    fn family(self, platform_fonts_only: bool) -> &'static str {
        match (self, platform_fonts_only) {
            (ReaderFont::Sans, _) => "system-ui, -apple-system, Roboto, sans-serif",
            (ReaderFont::Serif, true) => "ui-serif, serif",
            (ReaderFont::Serif, false) => r#""Noto Serif", Georgia, serif"#,
            (ReaderFont::Mono, true) => "ui-monospace, monospace",
            (ReaderFont::Mono, false) => {
                r#"ui-monospace, "SF Mono", SFMono-Regular, "Roboto Mono", monospace"#
            }
        }
    }
}

/// The CSS that sets the page's text variables, for a `<style>` in the page's `<head>`: the
/// typeface, and the size.
///
/// `text_scale` multiplies the page's 16px text, and everything sized relative to it. Make it the
/// reader's size setting times the system's text size, which is Dynamic Type on iOS and the font
/// scale on Android, so the post follows both without zooming the page. iOS's body text is 17pt,
/// so to match it, iOS's normal is `17.0 / 16.0`.
///
/// `colors` is the page's theme, since a theme that follows the system only uses the platform's
/// fonts.
#[uniffi::export]
pub fn reader_page_typography_css(
    font: ReaderFont,
    text_scale: f64,
    colors: ReaderPageColors,
) -> String {
    let scale = if text_scale.is_finite() && text_scale > 0.0 {
        text_scale.clamp(0.5, 4.0)
    } else {
        1.0
    };
    format!(
        ":root {{\n  --reader-font-family: {};\n  --reader-text-scale: {scale};\n}}\n",
        font.family(matches!(colors, ReaderPageColors::System { .. }))
    )
}

/// The CSS that sets the page's colour variables, for a `<style>` in the page's `<head>`.
///
/// For a theme with fixed colours, the CSS is the same in light and dark mode, so the web view's
/// appearance doesn't matter. That holds as long as the page's other styles take their colours from
/// these variables, and don't check `prefers-color-scheme` themselves.
#[uniffi::export]
pub fn reader_page_colors_css(colors: ReaderPageColors) -> String {
    match colors {
        ReaderPageColors::System { light, dark } => format!(
            ":root {{\n  color-scheme: light dark;\n{}}}\n\
             @media (prefers-color-scheme: dark) {{\n  :root {{\n{}  }}\n}}\n",
            variables(&light, "  "),
            variables(&dark, "    ")
        ),
        ReaderPageColors::Fixed { theme } => {
            // `color-scheme` makes the web view's own styling, like scrollbars and form controls,
            // match the theme rather than the system appearance.
            let scheme = if is_dark(theme) { "dark" } else { "light" };
            format!(
                ":root {{\n  color-scheme: {scheme};\n{}}}\n",
                variables(&reader_fixed_theme_colors(theme), "  ")
            )
        }
    }
}

fn variables(colors: &ReaderThemeColors, indent: &str) -> String {
    // No `..`: a new colour doesn't compile until it has a variable.
    let ReaderThemeColors {
        text,
        secondary_text,
        tertiary_text,
        background,
        secondary_background,
        border,
        link,
    } = *colors;

    [
        ("text", text),
        ("secondary-text", secondary_text),
        ("tertiary-text", tertiary_text),
        ("background", background),
        ("secondary-background", secondary_background),
        ("border", border),
        ("link", link),
    ]
    .into_iter()
    .map(|(name, color)| format!("{indent}--reader-{name}: {};\n", color.css()))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use rstest::rstest;

    #[rstest]
    #[case::soft(ReaderFixedTheme::Soft, "#2d2e2e", "#f2f2f2")]
    #[case::sepia(ReaderFixedTheme::Sepia, "#27201b", "#eae0cd")]
    #[case::evening(ReaderFixedTheme::Evening, "#abaab2", "#3a3a3c")]
    #[case::oled(ReaderFixedTheme::Oled, "#ffffff", "#000000")]
    #[case::hacker(ReaderFixedTheme::Hacker, "#00ff00", "#000000")]
    #[case::candy(ReaderFixedTheme::Candy, "#0066ff", "#ffe8fd")]
    fn fixed_themes_use_the_ios_colors(
        #[case] theme: ReaderFixedTheme,
        #[case] text: &str,
        #[case] background: &str,
    ) {
        let colors = reader_fixed_theme_colors(theme);
        assert_eq!(colors.text.css(), text);
        assert_eq!(colors.background.css(), background);
        assert_eq!(colors.link, colors.text);
    }

    #[rstest]
    #[case::light(
        ReaderFixedTheme::Sepia,
        "#27201b99",
        "#27201b54",
        "#27201b1a",
        "#27201b4d"
    )]
    #[case::dark(
        ReaderFixedTheme::Evening,
        "#abaab299",
        "#abaab273",
        "#abaab226",
        "#abaab24d"
    )]
    fn fixed_themes_derive_colors_from_the_text(
        #[case] theme: ReaderFixedTheme,
        #[case] secondary_text: &str,
        #[case] tertiary_text: &str,
        #[case] secondary_background: &str,
        #[case] border: &str,
    ) {
        let colors = reader_fixed_theme_colors(theme);
        assert_eq!(colors.secondary_text.css(), secondary_text);
        assert_eq!(colors.tertiary_text.css(), tertiary_text);
        assert_eq!(colors.secondary_background.css(), secondary_background);
        assert_eq!(colors.border.css(), border);
    }

    #[test]
    fn fixed_theme_css() {
        let css = reader_page_colors_css(ReaderPageColors::Fixed {
            theme: ReaderFixedTheme::Evening,
        });
        assert_eq!(
            css,
            indoc! {"
                :root {
                  color-scheme: dark;
                  --reader-text: #abaab2;
                  --reader-secondary-text: #abaab299;
                  --reader-tertiary-text: #abaab273;
                  --reader-background: #3a3a3c;
                  --reader-secondary-background: #abaab226;
                  --reader-border: #abaab24d;
                  --reader-link: #abaab2;
                }
            "}
        );
    }

    #[rstest]
    #[case::soft(ReaderFixedTheme::Soft, ReaderThemeAppearance::AlwaysLight)]
    #[case::sepia(ReaderFixedTheme::Sepia, ReaderThemeAppearance::AlwaysLight)]
    #[case::evening(ReaderFixedTheme::Evening, ReaderThemeAppearance::AlwaysDark)]
    #[case::oled(ReaderFixedTheme::Oled, ReaderThemeAppearance::AlwaysDark)]
    #[case::hacker(ReaderFixedTheme::Hacker, ReaderThemeAppearance::AlwaysDark)]
    #[case::candy(ReaderFixedTheme::Candy, ReaderThemeAppearance::AlwaysLight)]
    fn fixed_themes_always_look_the_same(
        #[case] theme: ReaderFixedTheme,
        #[case] appearance: ReaderThemeAppearance,
    ) {
        assert_eq!(ReaderPageColors::Fixed { theme }.appearance(), appearance);
    }

    #[test]
    fn the_system_theme_follows_the_system() {
        let colors = ReaderPageColors::System {
            light: reader_fixed_theme_colors(ReaderFixedTheme::Soft),
            dark: reader_fixed_theme_colors(ReaderFixedTheme::Oled),
        };
        assert_eq!(colors.appearance(), ReaderThemeAppearance::FollowsSystem);
    }

    fn system_colors() -> ReaderPageColors {
        ReaderPageColors::System {
            light: reader_fixed_theme_colors(ReaderFixedTheme::Soft),
            dark: reader_fixed_theme_colors(ReaderFixedTheme::Oled),
        }
    }

    fn sepia() -> ReaderPageColors {
        ReaderPageColors::Fixed {
            theme: ReaderFixedTheme::Sepia,
        }
    }

    #[rstest]
    #[case::sans(
        ReaderFont::Sans,
        1.0,
        sepia(),
        "system-ui, -apple-system, Roboto, sans-serif",
        "1"
    )]
    #[case::serif(
        ReaderFont::Serif,
        1.25,
        sepia(),
        r#""Noto Serif", Georgia, serif"#,
        "1.25"
    )]
    #[case::too_small(
        ReaderFont::Mono,
        0.1,
        sepia(),
        r#"ui-monospace, "SF Mono", SFMono-Regular, "Roboto Mono", monospace"#,
        "0.5"
    )]
    #[case::not_a_number(
        ReaderFont::Sans,
        f64::NAN,
        sepia(),
        "system-ui, -apple-system, Roboto, sans-serif",
        "1"
    )]
    // A theme that follows the system only uses the platform's fonts.
    #[case::sans_following_the_system(
        ReaderFont::Sans,
        1.0,
        system_colors(),
        "system-ui, -apple-system, Roboto, sans-serif",
        "1"
    )]
    #[case::serif_following_the_system(
        ReaderFont::Serif,
        1.0,
        system_colors(),
        "ui-serif, serif",
        "1"
    )]
    #[case::mono_following_the_system(
        ReaderFont::Mono,
        1.0,
        system_colors(),
        "ui-monospace, monospace",
        "1"
    )]
    fn typography_css(
        #[case] font: ReaderFont,
        #[case] text_scale: f64,
        #[case] colors: ReaderPageColors,
        #[case] family: &str,
        #[case] scale: &str,
    ) {
        assert_eq!(
            reader_page_typography_css(font, text_scale, colors),
            format!(
                ":root {{\n  --reader-font-family: {family};\n  --reader-text-scale: {scale};\n}}\n"
            )
        );
    }

    #[test]
    fn system_css() {
        let light = reader_fixed_theme_colors(ReaderFixedTheme::Soft);
        let dark = reader_fixed_theme_colors(ReaderFixedTheme::Oled);
        let css = reader_page_colors_css(ReaderPageColors::System { light, dark });
        assert_eq!(
            css,
            indoc! {"
                :root {
                  color-scheme: light dark;
                  --reader-text: #2d2e2e;
                  --reader-secondary-text: #2d2e2e99;
                  --reader-tertiary-text: #2d2e2e54;
                  --reader-background: #f2f2f2;
                  --reader-secondary-background: #2d2e2e1a;
                  --reader-border: #2d2e2e4d;
                  --reader-link: #2d2e2e;
                }
                @media (prefers-color-scheme: dark) {
                  :root {
                    --reader-text: #ffffff;
                    --reader-secondary-text: #ffffff99;
                    --reader-tertiary-text: #ffffff73;
                    --reader-background: #000000;
                    --reader-secondary-background: #ffffff26;
                    --reader-border: #ffffff4d;
                    --reader-link: #ffffff;
                  }
                }
            "}
        );
    }

    #[rstest]
    #[case::soft(ReaderFixedTheme::Soft, "light")]
    #[case::sepia(ReaderFixedTheme::Sepia, "light")]
    #[case::evening(ReaderFixedTheme::Evening, "dark")]
    #[case::oled(ReaderFixedTheme::Oled, "dark")]
    #[case::hacker(ReaderFixedTheme::Hacker, "dark")]
    #[case::candy(ReaderFixedTheme::Candy, "light")]
    fn fixed_theme_css_ignores_the_system_appearance(
        #[case] theme: ReaderFixedTheme,
        #[case] scheme: &str,
    ) {
        let css = reader_page_colors_css(ReaderPageColors::Fixed { theme });
        assert!(!css.contains("prefers-color-scheme"), "{css}");
        assert!(css.contains(&format!("color-scheme: {scheme};")), "{css}");
    }
}
