# wp_rs_web

A little web tool for debugging connection issues, and for previewing Reader posts.

## Reader Preview

`/reader` shows a post from the WordPress.com Reader the way the apps will. Paste a post's URL, a site's address, or a site ID. It fetches the post from the Reader API, cleans it up with `wp_mobile::reader`, and shows the page the apps' web views would, in any theme, at a phone's or tablet's width. Beside it is what the shared code found in the post: images, galleries, anchors, mentions, the embed scripts it loads, the excerpt and featured-image checks, and the HTML before and after the cleanup. Each post links to the same post in WordPress.com's Reader, to compare with, if you're logged in to WordPress.com.

It only works for public sites, because it calls the Reader API signed out. Each post page has the same Content Security Policy as the apps' pages, which only runs the scripts the apps would: the shared script, and the embeds' scripts the cleanup kept.

## Getting Started

First, build the project from the root directory using `cargo build`.

Then, from the `wp_rs_web` directory, run:

```
npm install
npm run build:css
```

Then, in another browser tab, run `cargo run --bin wp_rs_web` (also from the `wp_rs_web` directory).

Open http://127.0.0.1:8000.