use std::fmt;

use strum::IntoEnumIterator;

/// An sRGB colour.
///
/// To read one from the platform, use [`reader_color_from_argb`] for an Android colour int, or
/// [`reader_color_from_components`] for components between 0 and 1, like UIKit's. Both apps' native
/// wrappers have extensions for their own colour types.
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
}

#[uniffi::export]
impl ReaderColor {
    /// The colour in CSS hex: `#rrggbb`, or `#rrggbbaa` if it isn't opaque. As in CSS, the opacity
    /// comes last. Android's hex strings put it first, so use [`argb`](Self::argb) there.
    pub fn hex(&self) -> String {
        let Self {
            red,
            green,
            blue,
            alpha,
        } = *self;
        if alpha == u8::MAX {
            format!("#{red:02x}{green:02x}{blue:02x}")
        } else {
            format!("#{red:02x}{green:02x}{blue:02x}{alpha:02x}")
        }
    }

    /// The colour as an Android colour int (a `@ColorInt`): `0xAARRGGBB`, in a signed 32-bit
    /// integer, as Android stores it.
    pub fn argb(&self) -> i32 {
        i32::from_be_bytes([self.alpha, self.red, self.green, self.blue])
    }
}

/// A colour from CSS hex: `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa`, with or without the `#`. As in
/// CSS, the opacity comes last. Anything else, like a colour's name, gives `None`.
#[uniffi::export]
pub fn reader_color_from_hex(hex: &str) -> Option<ReaderColor> {
    let digits = hex.strip_prefix('#').unwrap_or(hex);
    if !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let channels: Vec<u8> = match digits.len() {
        // Each digit stands for two: `#f80` is `#ff8800`.
        3 | 4 => digits
            .chars()
            .map(|digit| digit.to_digit(16).map(|value| value as u8 * 17))
            .collect::<Option<_>>()?,
        6 | 8 => (0..digits.len())
            .step_by(2)
            .map(|start| u8::from_str_radix(&digits[start..start + 2], 16).ok())
            .collect::<Option<_>>()?,
        _ => return None,
    };
    Some(ReaderColor {
        red: channels[0],
        green: channels[1],
        blue: channels[2],
        alpha: channels.get(3).copied().unwrap_or(u8::MAX),
    })
}

/// A colour from an Android colour int (a `@ColorInt`), like `ContextCompat.getColor` gives:
/// `0xAARRGGBB`.
#[uniffi::export]
pub fn reader_color_from_argb(argb: i32) -> ReaderColor {
    let [alpha, red, green, blue] = argb.to_be_bytes();
    ReaderColor {
        red,
        green,
        blue,
        alpha,
    }
}

/// A colour from sRGB components between 0 and 1, like UIKit's `getRed(_:green:blue:alpha:)` gives.
/// Convert a colour from another colour space, like Display P3, to sRGB first.
///
/// Each component becomes the nearest of 256 steps. Ones outside 0 to 1, which an extended sRGB
/// colour can have, are clamped, and `NaN` is 0.
#[uniffi::export]
pub fn reader_color_from_components(red: f64, green: f64, blue: f64, alpha: f64) -> ReaderColor {
    let channel = |value: f64| {
        if value.is_nan() {
            0
        } else {
            (value.clamp(0.0, 1.0) * f64::from(u8::MAX)).round() as u8
        }
    };
    ReaderColor {
        red: channel(red),
        green: channel(green),
        blue: channel(blue),
        alpha: channel(alpha),
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
    /// In [`ReaderAppColors`], this is the app's brand colour: green for Jetpack, blue for
    /// WordPress. Pick shades that read as text on the background, at a contrast of 4.5:1 or more.
    /// The System and Reader themes use it. The other themes use their text colour for links
    /// instead, whatever the app.
    pub link: ReaderColor,
}

/// The colours only the app knows: the platform's own, in the system's light and dark
/// appearance, with the app's brand colour for links.
///
/// The System theme uses all of them, and the Reader theme takes its links' colour from them. The
/// other themes don't use them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Record)]
pub struct ReaderAppColors {
    pub light: ReaderThemeColors,
    pub dark: ReaderThemeColors,
}

/// The apps' brands. The System and Reader themes take their links' colour from the app's brand.
/// [`reader_brands`] lists them.
///
/// Its `Display` is the brand's name, like `Jetpack`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum, strum_macros::EnumIter)]
#[uniffi::export(Display)]
pub enum ReaderBrand {
    // Not `WordPress`, which would be `.wordPress` in Swift and `WORD_PRESS` in Kotlin.
    Wordpress,
    Jetpack,
}

impl ReaderBrand {
    fn id_str(self) -> &'static str {
        match self {
            Self::Wordpress => "wordpress",
            Self::Jetpack => "jetpack",
        }
    }

    /// The brand's colour for links. Pick shades that read as text on the background, at a
    /// contrast of 4.5:1 or more.
    fn link_color(self, dark: bool) -> ReaderColor {
        ReaderColor::rgb(match (self, dark) {
            // The WordPress app's blue.
            (Self::Wordpress, false) => 0x0675c4,
            (Self::Wordpress, true) => 0x399ce3,
            // Jetpack Green 50, as in Calypso's Jetpack colour scheme, and Green 30. The Green 40
            // the app uses elsewhere is too faint for text on white.
            (Self::Jetpack, false) => 0x008710,
            (Self::Jetpack, true) => 0x2fb41f,
        })
    }
}

impl fmt::Display for ReaderBrand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Wordpress => "WordPress",
            Self::Jetpack => "Jetpack",
        })
    }
}

#[uniffi::export]
impl ReaderBrand {
    /// A name for the brand that stays the same between releases, like `jetpack`.
    /// [`reader_brand_from_id`] reads it back.
    pub fn id(&self) -> String {
        self.id_str().to_owned()
    }
}

/// The apps' brands.
#[uniffi::export]
pub fn reader_brands() -> Vec<ReaderBrand> {
    ReaderBrand::iter().collect()
}

/// The brand whose [`id`](ReaderBrand::id) is `id`, if there is one.
#[uniffi::export]
pub fn reader_brand_from_id(id: &str) -> Option<ReaderBrand> {
    ReaderBrand::iter().find(|brand| brand.id_str() == id)
}

/// The app's colours for somewhere without the platform's own, like the Reader Preview or a test:
/// UIKit's system colours, with `brand`'s colour for links. The apps read theirs off the platform
/// instead, so the page matches the rest of the app.
#[uniffi::export]
pub fn reader_default_app_colors(brand: ReaderBrand) -> ReaderAppColors {
    let color = ReaderColor::rgb;
    ReaderAppColors {
        light: ReaderThemeColors {
            // `label`, `secondaryLabel` and `tertiaryLabel`.
            text: color(0x000000),
            secondary_text: color(0x3c3c43).with_opacity(0.6),
            tertiary_text: color(0x3c3c43).with_opacity(0.3),
            // `systemBackground` and `secondarySystemBackground`.
            background: color(0xffffff),
            secondary_background: color(0xf2f2f7),
            // `separator`.
            border: color(0x3c3c43).with_opacity(0.29),
            link: brand.link_color(false),
        },
        dark: ReaderThemeColors {
            text: color(0xffffff),
            secondary_text: color(0xebebf5).with_opacity(0.6),
            tertiary_text: color(0xebebf5).with_opacity(0.3),
            background: color(0x000000),
            secondary_background: color(0x1c1c1e),
            border: color(0x545458).with_opacity(0.6),
            link: brand.link_color(true),
        },
    }
}

/// The Reader's themes, in the order to show them in. [`reader_themes`] lists them, so the apps
/// don't keep a list of their own.
///
/// Its `Display` is the theme's name in English, like `OLED`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum, strum_macros::EnumIter)]
#[uniffi::export(Display)]
pub enum ReaderTheme {
    /// The default: the platform's own colours, from [`ReaderAppColors`], which follow the
    /// system's light or dark appearance.
    System,
    /// The WordPress.com Reader on the web: Calypso's colours, which follow the system's light or
    /// dark appearance, and its post text, with the app's colour for links.
    Reader,
    Soft,
    Sepia,
    Evening,
    Oled,
    Hacker,
    Candy,
}

impl ReaderTheme {
    fn id_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Reader => "reader",
            Self::Soft => "soft",
            Self::Sepia => "sepia",
            Self::Evening => "evening",
            Self::Oled => "oled",
            Self::Hacker => "hacker",
            Self::Candy => "candy",
        }
    }
}

impl fmt::Display for ReaderTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::System => "System",
            Self::Reader => "Reader",
            Self::Soft => "Soft",
            Self::Sepia => "Sepia",
            Self::Evening => "Evening",
            Self::Oled => "OLED",
            Self::Hacker => "Hacker",
            Self::Candy => "Candy",
        })
    }
}

#[uniffi::export]
impl ReaderTheme {
    /// A name for the theme that stays the same between releases, like `sepia`, for saving the
    /// reader's choice. [`reader_theme_from_id`] reads it back. It isn't for showing: the theme's
    /// `Display` is its name.
    pub fn id(&self) -> String {
        self.id_str().to_owned()
    }

    /// Whether the theme follows the system's light or dark appearance, or always looks the same.
    ///
    /// A theme that always looks the same ignores a forced appearance. Whether it's light or dark
    /// tells the app how to draw its own controls around the post.
    pub fn appearance(&self) -> ReaderThemeAppearance {
        match self {
            Self::System | Self::Reader => ReaderThemeAppearance::FollowsSystem,
            Self::Soft | Self::Sepia | Self::Candy => ReaderThemeAppearance::AlwaysLight,
            Self::Evening | Self::Oled | Self::Hacker => ReaderThemeAppearance::AlwaysDark,
        }
    }
}

/// The Reader's themes, in the order to show them in.
#[uniffi::export]
pub fn reader_themes() -> Vec<ReaderTheme> {
    ReaderTheme::iter().collect()
}

/// The theme whose [`id`](ReaderTheme::id) is `id`, if there is one.
#[uniffi::export]
pub fn reader_theme_from_id(id: &str) -> Option<ReaderTheme> {
    ReaderTheme::iter().find(|theme| theme.id_str() == id)
}

/// The colours `theme` draws the page with, in the system's light or dark appearance. The app draws
/// the background behind the web view, so it needs them too.
///
/// A theme that always looks the same has the same colours in both. Every one of its colours comes
/// from its text colour, except the background.
#[uniffi::export]
pub fn reader_theme_colors(
    theme: ReaderTheme,
    app_colors: ReaderAppColors,
    dark: bool,
) -> ReaderThemeColors {
    let app = if dark {
        app_colors.dark
    } else {
        app_colors.light
    };
    let (text, background) = match theme {
        ReaderTheme::System => return app,
        ReaderTheme::Reader => return calypso_colors(dark, app.link),
        ReaderTheme::Soft => (0x2d2e2e, 0xf2f2f2),
        ReaderTheme::Sepia => (0x27201b, 0xeae0cd),
        ReaderTheme::Evening => (0xabaab2, 0x3a3a3c),
        ReaderTheme::Oled => (0xffffff, 0x000000),
        ReaderTheme::Hacker => (0x00ff00, 0x000000),
        ReaderTheme::Candy => (0x0066ff, 0xffe8fd),
    };
    let text = ReaderColor::rgb(text);
    // Faint colours need a little more opacity to show up on a dark background.
    let is_dark = theme.appearance() == ReaderThemeAppearance::AlwaysDark;

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

/// The WordPress.com Reader's colours, from Calypso: its default colour scheme, with Color Studio's
/// grays, and its dark mode. Links are the app's colour.
fn calypso_colors(dark: bool, link: ReaderColor) -> ReaderThemeColors {
    let color = ReaderColor::rgb;
    if dark {
        // Calypso's dark mode, which maps these to the Dashboard's dark colours.
        return ReaderThemeColors {
            // `--dashboard__text-color`.
            text: color(0xe0e0e0),
            // `--dashboard__text-muted-color`.
            secondary_text: color(0xbdbdbd),
            // Calypso has no third text colour, so this is between the muted text and the page.
            tertiary_text: color(0x8a8a8a),
            // `--dashboard__background-color`.
            background: color(0x1e1e1e),
            // `--dashboard-surface__background-color`.
            secondary_background: color(0x2a2a2a),
            // `--dashboard-surface__border-color`.
            border: color(0x4a4a4a),
            link,
        };
    }
    ReaderThemeColors {
        // `--color-neutral-70`, which the post's text uses.
        text: color(0x3c434a),
        // `--color-text-subtle`.
        secondary_text: color(0x646970),
        tertiary_text: color(0x8c8f94),
        background: color(0xffffff),
        // `--color-neutral-0`.
        secondary_background: color(0xf6f7f7),
        // `--color-border-subtle`.
        border: color(0xdcdcde),
        link,
    }
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
/// `theme` is the page's theme, since a theme that follows the system only uses the platform's
/// fonts.
#[uniffi::export]
pub fn reader_page_typography_css(font: ReaderFont, text_scale: f64, theme: ReaderTheme) -> String {
    let scale = if text_scale.is_finite() && text_scale > 0.0 {
        text_scale.clamp(0.5, 4.0)
    } else {
        1.0
    };
    format!(
        ":root {{\n  --reader-font-family: {};\n  --reader-text-scale: {scale};\n}}\n",
        font.family(theme.appearance() == ReaderThemeAppearance::FollowsSystem)
    )
}

/// The WordPress.com Reader's post text, from Calypso's `.reader-full-post__story-content`.
const CALYPSO_TEXT_CSS: &str = r#"html {
  font-size: calc(17px * var(--reader-text-scale, 1));
}
body {
  max-width: 720px;
  margin: 0 auto;
  /* Calypso's 28px, as a ratio, so larger text, like a large paragraph, gets room too. */
  line-height: 1.647;
}
@media (max-width: 480px) {
  html {
    font-size: calc(16px * var(--reader-text-scale, 1));
  }
  body {
    line-height: 1.5;
  }
}
p {
  margin-bottom: 16px;
}
"#;

/// The CSS for the theme's text, besides its typeface and size, for a `<style>` after the shared
/// stylesheet. Only the Reader theme has any: the WordPress.com Reader's text size, line height and
/// width.
#[uniffi::export]
pub fn reader_page_theme_css(theme: ReaderTheme) -> String {
    match theme {
        ReaderTheme::Reader => CALYPSO_TEXT_CSS.to_owned(),
        ReaderTheme::System
        | ReaderTheme::Soft
        | ReaderTheme::Sepia
        | ReaderTheme::Evening
        | ReaderTheme::Oled
        | ReaderTheme::Hacker
        | ReaderTheme::Candy => String::new(),
    }
}

/// The CSS that sets the page's colour variables, for a `<style>` in the page's `<head>`.
///
/// For a theme that always looks the same, the CSS is the same in light and dark mode, so the web
/// view's appearance doesn't matter. That holds as long as the page's other styles take their
/// colours from these variables, and don't check `prefers-color-scheme` themselves.
#[uniffi::export]
pub fn reader_page_colors_css(theme: ReaderTheme, app_colors: ReaderAppColors) -> String {
    let colors = |dark| reader_theme_colors(theme, app_colors, dark);
    // `color-scheme` makes the web view's own styling, like scrollbars and form controls, match
    // the theme rather than the system appearance.
    let scheme = match theme.appearance() {
        ReaderThemeAppearance::FollowsSystem => {
            return format!(
                ":root {{\n  color-scheme: light dark;\n{}}}\n\
                 @media (prefers-color-scheme: dark) {{\n  :root {{\n{}  }}\n}}\n",
                variables(&colors(false), "  "),
                variables(&colors(true), "    ")
            );
        }
        ReaderThemeAppearance::AlwaysLight => "light",
        ReaderThemeAppearance::AlwaysDark => "dark",
    };
    format!(
        ":root {{\n  color-scheme: {scheme};\n{}}}\n",
        variables(&colors(scheme == "dark"), "  ")
    )
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
    .map(|(name, color)| format!("{indent}--reader-{name}: {};\n", color.hex()))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use rstest::rstest;

    #[rstest]
    #[case::short("#f80", (0xff, 0x88, 0x00, 0xff))]
    #[case::short_with_opacity("#f808", (0xff, 0x88, 0x00, 0x88))]
    #[case::long("#3858e9", (0x38, 0x58, 0xe9, 0xff))]
    #[case::long_with_opacity("#3858e980", (0x38, 0x58, 0xe9, 0x80))]
    #[case::upper_case("#3858E9", (0x38, 0x58, 0xe9, 0xff))]
    #[case::without_the_hash("3858e9", (0x38, 0x58, 0xe9, 0xff))]
    fn reads_css_hex(#[case] hex: &str, #[case] (red, green, blue, alpha): (u8, u8, u8, u8)) {
        assert_eq!(
            reader_color_from_hex(hex),
            Some(ReaderColor {
                red,
                green,
                blue,
                alpha
            })
        );
    }

    #[rstest]
    #[case::a_name("red")]
    #[case::too_short("#12")]
    #[case::five_digits("#12345")]
    #[case::too_long("#123456789")]
    #[case::not_hex("#gggggg")]
    #[case::not_ascii("#ééé")]
    #[case::empty("")]
    #[case::only_the_hash("#")]
    fn reads_nothing_from_other_text(#[case] hex: &str) {
        assert_eq!(reader_color_from_hex(hex), None);
    }

    #[rstest]
    #[case::opaque("#3858e9")]
    #[case::translucent("#3858e980")]
    fn hex_reads_back(#[case] hex: &str) {
        let color = reader_color_from_hex(hex).expect("the hex is valid");
        assert_eq!(color.hex(), hex);
    }

    #[test]
    fn reads_and_writes_android_color_ints() {
        let color = reader_color_from_hex("#3858e980").expect("the hex is valid");
        // Android's colour ints are signed, so a colour with its top bit set is negative.
        assert_eq!(color.argb(), 0x803858e9_u32 as i32);
        assert!(color.argb() < 0);
        assert_eq!(reader_color_from_argb(color.argb()), color);
        assert_eq!(
            reader_color_from_argb(0xff008710_u32 as i32).hex(),
            "#008710"
        );
    }

    #[rstest]
    #[case::exact(0.0, 1.0, 0.2, 1.0, (0, 255, 51, 255))]
    #[case::rounded(0.5, 0.502, 0.498, 0.6, (128, 128, 127, 153))]
    #[case::extended_srgb(-0.25, 1.1, 0.5, 2.0, (0, 255, 128, 255))]
    #[case::not_a_number(f64::NAN, 0.0, 0.0, 1.0, (0, 0, 0, 255))]
    fn reads_components_between_0_and_1(
        #[case] red: f64,
        #[case] green: f64,
        #[case] blue: f64,
        #[case] alpha: f64,
        #[case] expected: (u8, u8, u8, u8),
    ) {
        let color = reader_color_from_components(red, green, blue, alpha);
        assert_eq!((color.red, color.green, color.blue, color.alpha), expected);
    }

    #[rstest]
    #[case::wordpress(ReaderBrand::Wordpress, "wordpress", "WordPress", "#0675c4", "#399ce3")]
    #[case::jetpack(ReaderBrand::Jetpack, "jetpack", "Jetpack", "#008710", "#2fb41f")]
    fn each_brand_has_an_id_a_name_and_links(
        #[case] brand: ReaderBrand,
        #[case] id: &str,
        #[case] name: &str,
        #[case] light_link: &str,
        #[case] dark_link: &str,
    ) {
        assert_eq!(brand.id(), id);
        assert_eq!(reader_brand_from_id(id), Some(brand));
        assert_eq!(brand.to_string(), name);
        let colors = reader_default_app_colors(brand);
        assert_eq!(colors.light.link.hex(), light_link);
        assert_eq!(colors.dark.link.hex(), dark_link);
    }

    #[test]
    fn lists_the_brands() {
        assert_eq!(
            reader_brands(),
            [ReaderBrand::Wordpress, ReaderBrand::Jetpack]
        );
        assert_eq!(reader_brand_from_id("WordPress"), None);
    }

    #[test]
    fn lists_the_themes_in_order() {
        assert_eq!(
            reader_themes(),
            [
                ReaderTheme::System,
                ReaderTheme::Reader,
                ReaderTheme::Soft,
                ReaderTheme::Sepia,
                ReaderTheme::Evening,
                ReaderTheme::Oled,
                ReaderTheme::Hacker,
                ReaderTheme::Candy,
            ]
        );
    }

    #[rstest]
    #[case::system(ReaderTheme::System, "system", "System")]
    #[case::reader(ReaderTheme::Reader, "reader", "Reader")]
    #[case::soft(ReaderTheme::Soft, "soft", "Soft")]
    #[case::sepia(ReaderTheme::Sepia, "sepia", "Sepia")]
    #[case::evening(ReaderTheme::Evening, "evening", "Evening")]
    #[case::oled(ReaderTheme::Oled, "oled", "OLED")]
    #[case::hacker(ReaderTheme::Hacker, "hacker", "Hacker")]
    #[case::candy(ReaderTheme::Candy, "candy", "Candy")]
    fn each_theme_has_an_id_and_a_name(
        #[case] theme: ReaderTheme,
        #[case] id: &str,
        #[case] name: &str,
    ) {
        assert_eq!(theme.id(), id);
        assert_eq!(reader_theme_from_id(id), Some(theme));
        assert_eq!(theme.to_string(), name);
    }

    #[rstest]
    #[case::another_case("Sepia")]
    #[case::a_name("OLED")]
    #[case::empty("")]
    fn finds_no_theme_for_other_ids(#[case] id: &str) {
        assert_eq!(reader_theme_from_id(id), None);
    }

    #[rstest]
    #[case::soft(ReaderTheme::Soft, "#2d2e2e", "#f2f2f2")]
    #[case::sepia(ReaderTheme::Sepia, "#27201b", "#eae0cd")]
    #[case::evening(ReaderTheme::Evening, "#abaab2", "#3a3a3c")]
    #[case::oled(ReaderTheme::Oled, "#ffffff", "#000000")]
    #[case::hacker(ReaderTheme::Hacker, "#00ff00", "#000000")]
    #[case::candy(ReaderTheme::Candy, "#0066ff", "#ffe8fd")]
    fn fixed_themes_use_the_ios_colors(
        #[case] theme: ReaderTheme,
        #[case] text: &str,
        #[case] background: &str,
    ) {
        let colors = reader_theme_colors(
            theme,
            reader_default_app_colors(ReaderBrand::Wordpress),
            false,
        );
        assert_eq!(colors.text.hex(), text);
        assert_eq!(colors.background.hex(), background);
        assert_eq!(colors.link, colors.text);
        assert_eq!(
            reader_theme_colors(
                theme,
                reader_default_app_colors(ReaderBrand::Wordpress),
                true
            ),
            colors
        );
    }

    #[rstest]
    #[case::light(ReaderTheme::Sepia, "#27201b99", "#27201b54", "#27201b1a", "#27201b4d")]
    #[case::dark(
        ReaderTheme::Evening,
        "#abaab299",
        "#abaab273",
        "#abaab226",
        "#abaab24d"
    )]
    fn fixed_themes_derive_colors_from_the_text(
        #[case] theme: ReaderTheme,
        #[case] secondary_text: &str,
        #[case] tertiary_text: &str,
        #[case] secondary_background: &str,
        #[case] border: &str,
    ) {
        let colors = reader_theme_colors(
            theme,
            reader_default_app_colors(ReaderBrand::Wordpress),
            false,
        );
        assert_eq!(colors.secondary_text.hex(), secondary_text);
        assert_eq!(colors.tertiary_text.hex(), tertiary_text);
        assert_eq!(colors.secondary_background.hex(), secondary_background);
        assert_eq!(colors.border.hex(), border);
    }

    #[test]
    fn the_system_theme_uses_the_apps_colors() {
        let app = reader_default_app_colors(ReaderBrand::Wordpress);
        assert_eq!(
            reader_theme_colors(ReaderTheme::System, app, false),
            app.light
        );
        assert_eq!(
            reader_theme_colors(ReaderTheme::System, app, true),
            app.dark
        );
    }

    #[rstest]
    #[case::light(false, "#3c434a", "#ffffff")]
    #[case::dark(true, "#e0e0e0", "#1e1e1e")]
    fn the_reader_theme_uses_calypsos_colors_with_the_apps_links(
        #[case] dark: bool,
        #[case] text: &str,
        #[case] background: &str,
    ) {
        let app = reader_default_app_colors(ReaderBrand::Wordpress);
        let colors = reader_theme_colors(ReaderTheme::Reader, app, dark);
        assert_eq!(colors.text.hex(), text);
        assert_eq!(colors.background.hex(), background);
        let app_link = if dark { app.dark.link } else { app.light.link };
        assert_eq!(colors.link, app_link);
    }

    #[test]
    fn fixed_theme_css() {
        let css = reader_page_colors_css(
            ReaderTheme::Evening,
            reader_default_app_colors(ReaderBrand::Wordpress),
        );
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
    #[case::system(ReaderTheme::System, ReaderThemeAppearance::FollowsSystem)]
    #[case::reader(ReaderTheme::Reader, ReaderThemeAppearance::FollowsSystem)]
    #[case::soft(ReaderTheme::Soft, ReaderThemeAppearance::AlwaysLight)]
    #[case::sepia(ReaderTheme::Sepia, ReaderThemeAppearance::AlwaysLight)]
    #[case::evening(ReaderTheme::Evening, ReaderThemeAppearance::AlwaysDark)]
    #[case::oled(ReaderTheme::Oled, ReaderThemeAppearance::AlwaysDark)]
    #[case::hacker(ReaderTheme::Hacker, ReaderThemeAppearance::AlwaysDark)]
    #[case::candy(ReaderTheme::Candy, ReaderThemeAppearance::AlwaysLight)]
    fn each_theme_says_whether_it_follows_the_system(
        #[case] theme: ReaderTheme,
        #[case] appearance: ReaderThemeAppearance,
    ) {
        assert_eq!(theme.appearance(), appearance);
    }

    #[rstest]
    #[case::sans(
        ReaderFont::Sans,
        1.0,
        ReaderTheme::Sepia,
        "system-ui, -apple-system, Roboto, sans-serif",
        "1"
    )]
    #[case::serif(
        ReaderFont::Serif,
        1.25,
        ReaderTheme::Sepia,
        r#""Noto Serif", Georgia, serif"#,
        "1.25"
    )]
    #[case::too_small(
        ReaderFont::Mono,
        0.1,
        ReaderTheme::Sepia,
        r#"ui-monospace, "SF Mono", SFMono-Regular, "Roboto Mono", monospace"#,
        "0.5"
    )]
    #[case::not_a_number(
        ReaderFont::Sans,
        f64::NAN,
        ReaderTheme::Sepia,
        "system-ui, -apple-system, Roboto, sans-serif",
        "1"
    )]
    // A theme that follows the system only uses the platform's fonts.
    #[case::sans_following_the_system(
        ReaderFont::Sans,
        1.0,
        ReaderTheme::System,
        "system-ui, -apple-system, Roboto, sans-serif",
        "1"
    )]
    #[case::serif_following_the_system(
        ReaderFont::Serif,
        1.0,
        ReaderTheme::System,
        "ui-serif, serif",
        "1"
    )]
    #[case::mono_following_the_system(
        ReaderFont::Mono,
        1.0,
        ReaderTheme::System,
        "ui-monospace, monospace",
        "1"
    )]
    #[case::serif_in_the_reader_theme(
        ReaderFont::Serif,
        1.0,
        ReaderTheme::Reader,
        "ui-serif, serif",
        "1"
    )]
    fn typography_css(
        #[case] font: ReaderFont,
        #[case] text_scale: f64,
        #[case] theme: ReaderTheme,
        #[case] family: &str,
        #[case] scale: &str,
    ) {
        assert_eq!(
            reader_page_typography_css(font, text_scale, theme),
            format!(
                ":root {{\n  --reader-font-family: {family};\n  --reader-text-scale: {scale};\n}}\n"
            )
        );
    }

    #[test]
    fn only_the_reader_theme_has_text_css() {
        for theme in reader_themes() {
            let css = reader_page_theme_css(theme);
            assert_eq!(
                css.contains("font-size: calc(17px"),
                theme == ReaderTheme::Reader,
                "{theme}: {css}"
            );
            assert_eq!(css.is_empty(), theme != ReaderTheme::Reader, "{theme}");
        }
    }

    #[test]
    fn system_css() {
        let css = reader_page_colors_css(
            ReaderTheme::System,
            reader_default_app_colors(ReaderBrand::Wordpress),
        );
        assert_eq!(
            css,
            indoc! {"
                :root {
                  color-scheme: light dark;
                  --reader-text: #000000;
                  --reader-secondary-text: #3c3c4399;
                  --reader-tertiary-text: #3c3c434d;
                  --reader-background: #ffffff;
                  --reader-secondary-background: #f2f2f7;
                  --reader-border: #3c3c434a;
                  --reader-link: #0675c4;
                }
                @media (prefers-color-scheme: dark) {
                  :root {
                    --reader-text: #ffffff;
                    --reader-secondary-text: #ebebf599;
                    --reader-tertiary-text: #ebebf54d;
                    --reader-background: #000000;
                    --reader-secondary-background: #1c1c1e;
                    --reader-border: #54545899;
                    --reader-link: #399ce3;
                  }
                }
            "}
        );
    }

    #[rstest]
    #[case::soft(ReaderTheme::Soft, "light")]
    #[case::sepia(ReaderTheme::Sepia, "light")]
    #[case::evening(ReaderTheme::Evening, "dark")]
    #[case::oled(ReaderTheme::Oled, "dark")]
    #[case::hacker(ReaderTheme::Hacker, "dark")]
    #[case::candy(ReaderTheme::Candy, "light")]
    fn fixed_theme_css_ignores_the_system_appearance(
        #[case] theme: ReaderTheme,
        #[case] scheme: &str,
    ) {
        let css = reader_page_colors_css(theme, reader_default_app_colors(ReaderBrand::Wordpress));
        assert!(!css.contains("prefers-color-scheme"), "{css}");
        assert!(css.contains(&format!("color-scheme: {scheme};")), "{css}");
    }

    #[test]
    fn the_reader_theme_follows_the_system() {
        let css = reader_page_colors_css(
            ReaderTheme::Reader,
            reader_default_app_colors(ReaderBrand::Wordpress),
        );
        assert!(css.contains("color-scheme: light dark;"), "{css}");
        assert!(css.contains("--reader-background: #1e1e1e;"), "{css}");
    }
}
