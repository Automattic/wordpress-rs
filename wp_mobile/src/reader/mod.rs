//! The platform-neutral parts of rendering a Reader post, shared by the iOS and Android apps.
//!
//! Both apps show a Reader post in a web view. Anything that's HTML in, HTML out belongs here, so
//! both apps render the same page. Anything that needs the platform stays in the apps: the default
//! theme's colours (they come from UIKit and Material), and setting up the web view (base URL, link
//! handling, height measuring, appearance).
//!
//! So far, that's:
//!
//! - [`ReaderPostContent`], which cleans up post HTML from the Reader API, and finds what the app
//!   needs to know about it: images, galleries, mentions, anchors, scripts and text, how long it
//!   takes to read, whether it's cut off, and what to download ahead of time.
//! - [`reader_link`], which says what a link in a post goes to, so the app can open it the right
//!   way: in the post, in the Reader, in an image viewer or player, or in a browser.
//! - The theme colours: [`ReaderThemeColors`] is every colour the page uses, and
//!   [`reader_page_colors_css`] turns them into the page's CSS variables. The themes with fixed
//!   colours are defined here; the default theme's colours come from the app.
//! - [`reader_stylesheet`], which styles the post with those variables, and [`reader_script`], which
//!   fades images in and passes taps and text selection to the app.
//! - [`ReaderPostContent::page_html`], which puts all of them together into the page for the web
//!   view, with [`ReaderPageOptions`] for the theme, the site's language, reading offline, and the
//!   app's own CSS.

mod anchors;
mod assets;
mod clean;
mod content;
mod embeds;
mod getty;
mod gists;
mod images;
mod links;
mod page;
mod shortcodes;
mod text;
mod theme;

pub use anchors::{ReaderPostAnchor, ReaderPostAnchorKind};
pub use assets::{reader_script, reader_stylesheet};
pub use content::{
    ReaderPostContent, ReaderPostGallery, ReaderPostGalleryPosition, ReaderPostMention,
};
pub use images::{ReaderImageSize, ReaderPostImage};
pub use links::{ReaderLink, reader_link};
pub use page::{ReaderPageOptions, reader_page_base_url};
pub use theme::{
    ReaderColor, ReaderFixedTheme, ReaderFont, ReaderPageColors, ReaderThemeAppearance,
    ReaderThemeColors, reader_fixed_theme_colors, reader_fixed_theme_from_id, reader_fixed_themes,
    reader_page_colors_css, reader_page_typography_css,
};

#[cfg(test)]
mod corpus_tests;
#[cfg(test)]
mod safety_tests;
#[cfg(test)]
mod snapshot_tests;
#[cfg(test)]
mod test_site_tests;
