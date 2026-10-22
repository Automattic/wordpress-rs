use std::{collections::BTreeSet, sync::LazyLock};

use scraper::{ElementRef, Html, Selector};
use url::Url;

/// An embed whose markup only turns into the embed once the provider's script runs.
struct KnownEmbed {
    /// Matches the embed's markup.
    selector: &'static str,
    /// The script to load when the post doesn't include one.
    script: &'static str,
    /// The hosts the provider serves its script from. A post script from any of them will do.
    script_hosts: &'static [&'static str],
    /// Other hosts the provider's script loads scripts from, which the page's Content Security
    /// Policy has to allow too.
    more_script_hosts: &'static [&'static str],
}

/// The embeds the apps load scripts for, even when the post doesn't include one. The Reader API
/// removes every script from posts it reads from a site's feed, as it does for many Jetpack sites,
/// so these are the only embeds that work in those posts.
const KNOWN_EMBEDS: &[KnownEmbed] = &[
    KnownEmbed {
        selector: r#"blockquote[class^="instagram-"]"#,
        script: "https://www.instagram.com/embed.js",
        script_hosts: &["www.instagram.com", "platform.instagram.com"],
        more_script_hosts: &[],
    },
    KnownEmbed {
        selector: r#"blockquote[class^="twitter-"], a[class^="twitter-"]"#,
        script: "https://platform.twitter.com/widgets.js",
        script_hosts: &["platform.twitter.com", "platform.x.com"],
        more_script_hosts: &[],
    },
    KnownEmbed {
        selector: r#"fb\:post, [class^="fb-"]"#,
        script: "https://connect.facebook.net/en_US/sdk.js#xfbml=1&version=v2.8",
        script_hosts: &["connect.facebook.net"],
        more_script_hosts: &[],
    },
    KnownEmbed {
        selector: r#"[class^="tumblr-"]"#,
        script: "https://assets.tumblr.com/post.js",
        script_hosts: &["assets.tumblr.com", "secure.assets.tumblr.com"],
        more_script_hosts: &[],
    },
    KnownEmbed {
        // Posts from a site's feed don't have WordPress.com's `.embed-reddit` wrapper. Older embed
        // code is a `.reddit-card`, which the script handles too.
        selector: ".embed-reddit, blockquote.reddit-embed-bq, blockquote.reddit-card",
        script: "https://embed.reddit.com/widgets.js",
        script_hosts: &["embed.reddit.com", "embed.redditmedia.com"],
        more_script_hosts: &[],
    },
    KnownEmbed {
        selector: "blockquote.bluesky-embed",
        script: "https://embed.bsky.app/static/embed.js",
        script_hosts: &["embed.bsky.app"],
        more_script_hosts: &[],
    },
    KnownEmbed {
        // Jetpack's Pinterest embed is an empty link until the script turns it into a board or pin. Its
        // "Pin it" buttons are `data-pin-do="buttonPin"`, and don't need it.
        selector: r#"a[data-pin-do^="embed"]"#,
        script: "https://assets.pinterest.com/js/pinit.js",
        script_hosts: &["assets.pinterest.com"],
        // A board's pins, which `pinit.js` loads as a script.
        more_script_hosts: &["widgets.pinterest.com"],
    },
    KnownEmbed {
        // TikTok's embed code. TikTok's oEmbed now returns a player iframe instead, which needs no
        // script, so WordPress.com's `.embed-tiktok` wrapper doesn't mean there's one of these.
        // Posts from a site's feed have the embed code without the wrapper.
        selector: "blockquote.tiktok-embed",
        script: "https://www.tiktok.com/embed.js",
        script_hosts: &["www.tiktok.com"],
        // `embed.js` redirects to a regional CDN, and loads TikTok's embed library from it: the
        // first in the US, the second elsewhere.
        more_script_hosts: &[
            "lf16-tiktok-web.tiktokcdn-us.com",
            "sf16-website-login.neutral.ttwstatic.com",
        ],
    },
    KnownEmbed {
        // WordPress.com has no Threads embed, so these are the embed code from Threads, pasted into
        // a post. The script turns each into an iframe, and loads no other scripts.
        selector: "blockquote.text-post-media",
        script: "https://www.threads.net/embed.js",
        script_hosts: &["www.threads.net", "www.threads.com"],
        more_script_hosts: &[],
    },
];

static SELECTORS: LazyLock<Vec<Selector>> = LazyLock::new(|| {
    KNOWN_EMBEDS
        .iter()
        .map(|embed| Selector::parse(embed.selector).expect("known embed selectors are valid"))
        .collect()
});

/// The scripts for the known embeds in `document` that none of `scripts` comes from.
pub(crate) fn missing_scripts(document: &Html, scripts: &[String]) -> Vec<&'static str> {
    let hosts: Vec<String> = scripts
        .iter()
        .filter_map(|script| Url::parse(script).ok()?.host_str().map(str::to_owned))
        .collect();

    KNOWN_EMBEDS
        .iter()
        .zip(SELECTORS.iter())
        .filter(|(embed, _)| {
            !embed
                .script_hosts
                .iter()
                .any(|host| hosts.iter().any(|existing| existing == *host))
        })
        .filter(|(_, selector)| document.select(selector).next().is_some())
        .map(|(embed, _)| embed.script)
        .collect()
}

/// Whether `element` is a known embed's markup. Some show nothing until the script runs, like
/// Pinterest's empty link.
pub(crate) fn is_known_embed(element: ElementRef) -> bool {
    SELECTORS.iter().any(|selector| selector.matches(&element))
}

/// An embed whose script the page loads only when the post includes it.
struct OtherEmbed {
    /// The hosts the provider serves its script from.
    script_hosts: &'static [&'static str],
    /// Other hosts the provider's script loads scripts from.
    more_script_hosts: &'static [&'static str],
}

/// Other embeds, whose scripts the page loads when the post includes them.
///
/// Getty's `widgets.js` isn't one. It needs the inline script the cleanup removes, and throws
/// without it, so the cleanup builds Getty's iframe from that script instead.
const OTHER_EMBEDS: &[OtherEmbed] = &[
    OtherEmbed {
        script_hosts: &["embedr.flickr.com"],
        // The embed itself, which `client-code.js` loads through `embedr-loader.js`. Without it the
        // post's plain image stays.
        more_script_hosts: &["widgets.flickr.com"],
    },
    OtherEmbed {
        script_hosts: &["s.imgur.com"],
        // `embed.js` loads its controller from `imgur.com` on any other site.
        more_script_hosts: &["imgur.com"],
    },
];

/// Each provider's script hosts, with the other hosts its scripts load scripts from.
fn providers() -> impl Iterator<Item = (&'static [&'static str], &'static [&'static str])> {
    KNOWN_EMBEDS
        .iter()
        .map(|embed| (embed.script_hosts, embed.more_script_hosts))
        .chain(
            OTHER_EMBEDS
                .iter()
                .map(|embed| (embed.script_hosts, embed.more_script_hosts)),
        )
}

/// Whether the page should load a script the post includes. Only embeds' scripts qualify: posts
/// also include ads, trackers and scripts that only work on their own site.
///
/// That leaves out VideoPress's iframe script too, which can hide the player for good when it hits
/// an error. The player works without it.
pub(crate) fn is_allowed(script: &Url) -> bool {
    let Some(host) = script.host_str() else {
        return false;
    };
    providers()
        .flat_map(|(script_hosts, _)| script_hosts)
        .any(|allowed| host.eq_ignore_ascii_case(allowed))
}

/// The hosts a page that loads `scripts` lets scripts load from: every host each script's provider
/// serves scripts from. Embed scripts load more of their own.
pub(crate) fn policy_hosts(scripts: &[String]) -> BTreeSet<String> {
    let mut hosts = BTreeSet::new();
    for host in scripts.iter().filter_map(|script| {
        Url::parse(script)
            .ok()?
            .host_str()
            .map(str::to_ascii_lowercase)
    }) {
        let provider = providers().find(|(script_hosts, _)| {
            script_hosts
                .iter()
                .any(|allowed| host.eq_ignore_ascii_case(allowed))
        });
        match provider {
            Some((script_hosts, more_script_hosts)) => hosts.extend(
                script_hosts
                    .iter()
                    .chain(more_script_hosts)
                    .map(|host| host.to_string()),
            ),
            None => {
                hosts.insert(host);
            }
        }
    }
    hosts
}

#[cfg(test)]
mod tests {
    use crate::reader::ReaderPostContent;
    use rstest::rstest;

    const FACEBOOK: &str = "https://connect.facebook.net/en_US/sdk.js#xfbml=1&version=v2.8";

    #[rstest]
    #[case::instagram(
        r#"<blockquote class="instagram-media"></blockquote>"#,
        &["https://www.instagram.com/embed.js"]
    )]
    #[case::twitter_quote(
        r#"<blockquote class="twitter-tweet"></blockquote>"#,
        &["https://platform.twitter.com/widgets.js"]
    )]
    #[case::twitter_link(
        r#"<a class="twitter-timeline" href="https://x.com/WordPress">Posts</a>"#,
        &["https://platform.twitter.com/widgets.js"]
    )]
    #[case::facebook_tag("<fb:post></fb:post>", &[FACEBOOK])]
    #[case::facebook_class(r#"<div class="fb-post"></div>"#, &[FACEBOOK])]
    #[case::tumblr(
        r#"<div class="tumblr-post"></div>"#,
        &["https://assets.tumblr.com/post.js"]
    )]
    #[case::reddit_on_wordpress_com(
        r#"<div class="embed-reddit"><blockquote class="reddit-embed-bq"></blockquote></div>"#,
        &["https://embed.reddit.com/widgets.js"]
    )]
    #[case::reddit(
        r#"<blockquote class="reddit-embed-bq"></blockquote>"#,
        &["https://embed.reddit.com/widgets.js"]
    )]
    #[case::older_reddit(
        r#"<blockquote class="reddit-card"></blockquote>"#,
        &["https://embed.reddit.com/widgets.js"]
    )]
    #[case::bluesky(
        r#"<blockquote class="bluesky-embed" data-bluesky-uri="at://did:plc:a/app.bsky.feed.post/b"></blockquote>"#,
        &["https://embed.bsky.app/static/embed.js"]
    )]
    #[case::tiktok_on_wordpress_com(
        r#"<div class="embed-tiktok"><blockquote class="tiktok-embed" data-video-id="1"></blockquote></div>"#,
        &["https://www.tiktok.com/embed.js"]
    )]
    #[case::tiktok(
        r#"<blockquote class="tiktok-embed" data-video-id="1"></blockquote>"#,
        &["https://www.tiktok.com/embed.js"]
    )]
    #[case::tiktok_player(
        r#"<div class="embed-tiktok"><iframe src="https://www.tiktok.com/player/v1/1"></iframe></div>"#,
        &[]
    )]
    #[case::pinterest(
        r#"<a data-pin-do="embedBoard" href="https://www.pinterest.com/pinterest/official-news/"></a>"#,
        &["https://assets.pinterest.com/js/pinit.js"]
    )]
    #[case::threads(
        r#"<blockquote class="text-post-media" data-text-post-permalink="https://www.threads.com/@a/post/B"><a href="https://www.threads.com/@a/post/B">View on Threads</a></blockquote>"#,
        &["https://www.threads.net/embed.js"]
    )]
    #[case::no_embeds("<p>Hi</p>", &[])]
    fn adds_scripts_for_known_embeds(#[case] html: &str, #[case] expected: &[&str]) {
        assert_eq!(ReaderPostContent::new(html, None).scripts(), expected);
    }

    #[rstest]
    #[case::same_script(
        r#"<blockquote class="twitter-tweet"></blockquote><script src="https://platform.twitter.com/widgets.js"></script>"#,
        &["https://platform.twitter.com/widgets.js"]
    )]
    #[case::other_host_for_the_same_provider(
        r#"<blockquote class="twitter-tweet"></blockquote><script src="https://platform.x.com/widgets.js"></script>"#,
        &["https://platform.x.com/widgets.js"]
    )]
    #[case::other_path_on_the_same_host(
        r#"<blockquote class="instagram-media"></blockquote><script src="//platform.instagram.com/en_US/embeds.js"></script>"#,
        &["https://platform.instagram.com/en_US/embeds.js"]
    )]
    #[case::older_tumblr_host(
        r#"<div class="tumblr-post"></div><script async src="https://secure.assets.tumblr.com/post.js"></script>"#,
        &["https://secure.assets.tumblr.com/post.js"]
    )]
    #[case::post_scripts_come_first(
        r#"<blockquote class="instagram-media"></blockquote><script src="https://embedr.flickr.com/assets/client-code.js"></script>"#,
        &["https://embedr.flickr.com/assets/client-code.js", "https://www.instagram.com/embed.js"]
    )]
    fn uses_the_post_script_for_an_embed(#[case] html: &str, #[case] expected: &[&str]) {
        assert_eq!(ReaderPostContent::new(html, None).scripts(), expected);
    }

    #[rstest]
    #[case::none(&[], &[])]
    #[case::every_host_the_provider_uses(
        &["https://platform.x.com/widgets.js"],
        &["platform.twitter.com", "platform.x.com"]
    )]
    #[case::hosts_the_script_loads_more_from(
        &["https://assets.pinterest.com/js/pinit.js"],
        &["assets.pinterest.com", "widgets.pinterest.com"]
    )]
    #[case::tiktok_regional_cdns(
        &["https://www.tiktok.com/embed.js"],
        &[
            "lf16-tiktok-web.tiktokcdn-us.com",
            "sf16-website-login.neutral.ttwstatic.com",
            "www.tiktok.com"
        ]
    )]
    #[case::other_embeds(
        &["https://embedr.flickr.com/assets/client-code.js", FACEBOOK],
        &["connect.facebook.net", "embedr.flickr.com", "widgets.flickr.com"]
    )]
    #[case::imgur(
        &["https://s.imgur.com/min/embed.js"],
        &["imgur.com", "s.imgur.com"]
    )]
    fn lists_the_hosts_a_page_loads_scripts_from(
        #[case] scripts: &[&str],
        #[case] expected: &[&str],
    ) {
        let scripts: Vec<String> = scripts.iter().map(|script| script.to_string()).collect();
        let hosts: Vec<String> = super::policy_hosts(&scripts).into_iter().collect();
        assert_eq!(hosts, expected);
    }
}
