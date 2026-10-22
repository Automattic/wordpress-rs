import WordPressAPIInternal

// The Reader's shared rendering functions, from `wp_mobile::reader`. A `typealias` can't export a
// function, so each one calls its counterpart in `WordPressAPIInternal`. Their types are exported in
// `Exports.swift`.

// MARK: - The page

/// The shared stylesheet for a Reader post page.
public func readerStylesheet() -> String {
    WordPressAPIInternal.readerStylesheet()
}

/// The shared script for a Reader post page.
public func readerScript() -> String {
    WordPressAPIInternal.readerScript()
}

/// The base URL to load a Reader post page with.
public func readerPageBaseUrl() -> String {
    WordPressAPIInternal.readerPageBaseUrl()
}

/// What a link in a post goes to, so the app can open it the right way. `postUrl` is the post's
/// own address, for links within it.
public func readerLink(url: String, postUrl: String? = nil) -> ReaderLink {
    WordPressAPIInternal.readerLink(url: url, postUrl: postUrl)
}

// MARK: - Themes

/// The Reader's themes, in the order to show them in.
public func readerThemes() -> [ReaderTheme] {
    WordPressAPIInternal.readerThemes()
}

/// The theme with the ID `ReaderTheme.id()` gives, if there is one.
public func readerThemeFromId(id: String) -> ReaderTheme? {
    WordPressAPIInternal.readerThemeFromId(id: id)
}

/// The colours `theme` draws the page with, in the system's light or dark appearance.
public func readerThemeColors(theme: ReaderTheme, appColors: ReaderAppColors, dark: Bool) -> ReaderThemeColors {
    WordPressAPIInternal.readerThemeColors(theme: theme, appColors: appColors, dark: dark)
}

/// The CSS that sets the page's colour variables, for a `<style>` in the page's `<head>`.
public func readerPageColorsCss(theme: ReaderTheme, appColors: ReaderAppColors) -> String {
    WordPressAPIInternal.readerPageColorsCss(theme: theme, appColors: appColors)
}

/// The CSS that sets the page's typeface and text size, for a `<style>` in the page's `<head>`.
public func readerPageTypographyCss(font: ReaderFont, textScale: Double, theme: ReaderTheme) -> String {
    WordPressAPIInternal.readerPageTypographyCss(font: font, textScale: textScale, theme: theme)
}

/// The CSS for the theme's text, for a `<style>` after the shared stylesheet. Only the Reader theme
/// has any.
public func readerPageThemeCss(theme: ReaderTheme) -> String {
    WordPressAPIInternal.readerPageThemeCss(theme: theme)
}

// MARK: - Brands and colours

/// The apps' brands.
public func readerBrands() -> [ReaderBrand] {
    WordPressAPIInternal.readerBrands()
}

/// The brand with the ID `ReaderBrand.id()` gives, if there is one.
public func readerBrandFromId(id: String) -> ReaderBrand? {
    WordPressAPIInternal.readerBrandFromId(id: id)
}

/// The app's colours for somewhere without the platform's own, like a preview or a test: UIKit's
/// system colours, with `brand`'s colour for links. Read the app's own colours off the platform
/// instead, so the page matches the rest of the app.
public func readerDefaultAppColors(brand: ReaderBrand) -> ReaderAppColors {
    WordPressAPIInternal.readerDefaultAppColors(brand: brand)
}

/// A colour from CSS hex: `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa`. As in CSS, the opacity comes
/// last.
public func readerColorFromHex(hex: String) -> ReaderColor? {
    WordPressAPIInternal.readerColorFromHex(hex: hex)
}

/// A colour from an Android colour int: `0xAARRGGBB`.
public func readerColorFromArgb(argb: Int32) -> ReaderColor {
    WordPressAPIInternal.readerColorFromArgb(argb: argb)
}

/// A colour from sRGB components between 0 and 1. To read a `CGColor` or `UIColor` in any colour
/// space, use `ReaderColor`'s initializers instead.
public func readerColorFromComponents(red: Double, green: Double, blue: Double, alpha: Double) -> ReaderColor {
    WordPressAPIInternal.readerColorFromComponents(red: red, green: green, blue: blue, alpha: alpha)
}
