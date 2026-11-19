# WordPress.com Reader endpoints

Every WordPress.com Reader endpoint the library has: what it's for, the parameters the library sends, and what WordPress.com's public documentation declares for it.

This file is written by `scripts/reader-endpoints-doc.py`. Don't edit it by hand: change the comment in the endpoint file or on the parameter's field, and run the script again. `Documentation/adding-a-wordpress-com-endpoint.md` says how an endpoint is added.

## Where this comes from

- **The library's own code.** Each endpoint's description is the comment on its request in `wp_api/src/wp_com/endpoint/reader_*_endpoint.rs`. Each parameter's is the comment on its field. They say what was seen when the endpoint was called, not what its code says.
- **WordPress.com's public documentation of itself**: `https://public-api.wordpress.com/rest/v1.2/help` (and `v1.1`, `v1.3`) for the `rest/v1.x` endpoints, and `https://public-api.wordpress.com/wpcom/v2/` for the `wpcom/v2` routes. Neither needs a token. Neither lists every endpoint: one that isn't there says so below.

Every `rest/v1.x` endpoint also declares `context`, `http_envelope`, `pretty`, `meta`, `fields` and `callback`. They're left out below unless the library sends them.

## Contents

- [Streams of posts](#streams-of-posts)
- [Single posts](#single-posts)
- [Sites and feeds](#sites-and-feeds)
- [Tags and the menu](#tags-and-the-menu)
- [Users](#users)
- [Saved posts](#saved-posts)
- [Seen status](#seen-status)
- [Likes](#likes)
- [Subscriptions](#subscriptions)
- [Blocked sites](#blocked-sites)
- [Comments](#comments)

## Streams of posts

`wp_api/src/wp_com/endpoint/reader_posts_endpoint.rs`

### `GET /rest/v1.2/read/following`

Posts from the sites and feeds the user follows.

Request `ReaderPostsRequest::Following`, response `ReaderPostsResponse`.

Parameters the library sends, in the query (`ReaderStreamParams`):

- `number` (`u32`): The number of posts to return. The apps ask for 7.
- `before` (`WpGmtDateTime`): Only return posts before this time.
- `after` (`WpGmtDateTime`): Only return posts after this time.
- `meta` (`String`): Extra data to include, comma-separated: `site` sends each post's site, which has its `ReaderPost::site_locale`, and `likes` its likes.
- `fields` (`String`): The fields of each post to return, comma-separated, such as `ID,site_ID,date`. Without it a post arrives with every field, about 60 of them.
  - `ID` has to be among them. A post without one can't be read, and is listed in `ReaderPostsResponse::unreadable_posts`.
  - `meta` has to be among them for what `meta` asks for to arrive.
  - Paging by date needs `date`.
  - The API works some fields out from others, and only from the ones asked for. It fills `featured_media` from a post's `attachments` and `content`, so without those it's empty.

  A stream with no posts then answers with an empty list where the page would be, which reads as a page with no posts.

Public documentation: `rest/v1.1/help`, which documents the `rest/v1.1` endpoint only. It declares `after`, `before`, `fields`, `meta`, `number`, which the library sends.

Declared there, and not sent by the library:

- `page`: (int) Return the Nth 1-indexed page of posts.
- `order`: (string) One of: `DESC` ((default) Return posts in descending order. For dates, that means newest to oldest.); `ASC` (Return posts in ascending order. For dates, that means oldest to newest.)
- `feed_id`: (int) The feed_id to fetch the posts from.

### `GET /rest/v1.2/read/sites/$site/posts`

A WordPress.com site's posts.

Request `ReaderPostsRequest::SitePosts`, response `ReaderPostsResponse`.

Parameters the library sends, in the query (`ReaderStreamParams`): `number`, `before`, `after`, `meta`, `fields`. They're described under `GET /rest/v1.2/read/following`.

Public documentation: not listed.

### `GET /rest/v1.2/read/feed/$feed/posts`

An RSS feed's posts.

Request `ReaderPostsRequest::FeedPosts`, response `ReaderPostsResponse`.

Parameters the library sends, in the query (`ReaderStreamParams`): `number`, `before`, `after`, `meta`, `fields`. They're described under `GET /rest/v1.2/read/following`.

Public documentation: not listed.

### `GET /wpcom/v2/read/tags/$tag/posts`

A tag's posts.

Request `ReaderPostsRequest::TagPosts`, response `ReaderTagPostsResponse`.

Parameters the library sends, in the query (`ReaderTagPostsParams`):

- `number` (`u32`): The number of posts to return. The API returns 10 at most, whatever is asked for.
- `page_handle` (`String`): The previous page's `ReaderTagPostsResponse::next_page_handle`, from a page sorted by date. A handle from a page sorted by popularity returns the first page again.
- `page` (`u32`): The page to return, counting from 1. It works in either order, and it's the only way to page posts sorted by popularity.
- `sort` (`ReaderTagPostsSort`): The order of the posts. The API's default is `ReaderTagPostsSort::Date`.
- `before` (`WpGmtDateTime`): Only return posts published before this time.
- `after` (`WpGmtDateTime`): Only return posts published after this time.

Public documentation: the `wpcom/v2` route index. It declares `after`, `before`, `number`, `page`, `page_handle`, `sort`, which the library sends.

Declared there, and not sent by the library:

- `_locale`: (string) Locale of the client.
- `tag`: (string) Tag to fetch posts for.

### `GET /rest/v1.2/read/liked`

The posts the user liked, newest like first. Signed out, the API answers 403.

Request `ReaderPostsRequest::Liked`, response `ReaderPostsResponse`.

Parameters the library sends, in the query (`ReaderLikedPostsParams`):

- `number` (`u32`): The number of posts to return: 10 by default, and at most 40.
- `before` (`WpGmtDateTime`): Only return posts liked before this time. A post liked at that very second is left out.
- `after` (`WpGmtDateTime`): Only return posts liked after this time. A post liked at that very second is left out.
- `order` (`ReaderStreamOrder`): The order of the posts. `ReaderStreamOrder::Asc` starts from the oldest like, and a page's `date_range` then has the last post's date in `before`. To page that way, pass that date as `after`.
- `page` (`u32`): The page to return, counting from 1. Don't page with it: each page starts 5 posts later than it should, so those posts are never returned. Page with `before`.
- `meta` (`String`): Extra data to include, comma-separated, as in `ReaderStreamParams::meta`.
- `fields` (`String`): The fields of each post to return, comma-separated, as in `ReaderStreamParams::fields`. The page's `date_range` arrives whichever fields are asked for, and a stream with no posts still answers with a page, not an empty list.

Public documentation: `rest/v1.1/help`, which documents the `rest/v1.1` endpoint only. It declares `after`, `before`, `fields`, `meta`, `number`, `order`, `page`, which the library sends.

### `GET /rest/v1.2/read/a8c`

Posts from Automattic's sites, for Automatticians. Signed out, the API answers with a page with no posts.

Request `ReaderPostsRequest::Automattic`, response `ReaderPostsResponse`.

Parameters the library sends, in the query (`ReaderOrganizationPostsParams`):

- `number` (`u32`): The number of posts to return: 10 by default, and at most 40.
- `before` (`WpGmtDateTime`): Only return posts published before this time. A post published at that very second is left out.
- `after` (`WpGmtDateTime`): Only return posts published after this time. A post published at that very second is left out.
- `order` (`ReaderStreamOrder`): The order of the posts on the page. `ReaderStreamOrder::Asc` returns the same posts as `ReaderStreamOrder::Desc`, the newest ones, with the oldest of them first. It doesn't start the stream from its oldest post.
- `page` (`u32`): The page to return, counting from 1, from the newest post at the time of the request. A new post moves every other post along by one, so page with `before` instead.
- `meta` (`String`): Extra data to include, comma-separated, as in `ReaderStreamParams::meta`.
- `fields` (`String`): The fields of each post to return, comma-separated, as in `ReaderStreamParams::fields`. The page's `date_range` arrives whichever fields are asked for, and a stream with no posts still answers with a page, not an empty list.

Public documentation: not listed.

### `GET /rest/v1.2/read/following/p2`

Posts from the P2s the user follows. Signed out, the API answers 403.

Request `ReaderPostsRequest::FollowingP2`, response `ReaderPostsResponse`.

Parameters the library sends, in the query (`ReaderOrganizationPostsParams`): `number`, `before`, `after`, `order`, `page`, `meta`, `fields`. They're described under `GET /rest/v1.2/read/a8c`.

Public documentation: not listed.

### `GET /rest/v1.2/freshly-pressed`

Freshly Pressed: the posts WordPress.com's editors picked. Answers signed out.

Request `ReaderPostsRequest::FreshlyPressed`, response `ReaderPostsResponse`.

Parameters the library sends, in the query (`ReaderFreshlyPressedParams`):

- `number` (`u32`): The number of posts to return: 10 by default, and at most 40.
- `before` (`WpGmtDateTime`): Only return posts shown before this time. A post shown at that very second is left out.
- `after` (`WpGmtDateTime`): Only return posts shown at or after this time.
- `content_width` (`u32`): The width, in pixels, to size the images in each post's content for. 480 by default.
- `thumb_width` (`u32`): The width, in pixels, of the screenshot in `editorial.image`. 252 by default.
- `thumb_height` (`u32`): The height, in pixels, of thumbnail images. 160 by default. It changed nothing in the responses that were looked at.
- `meta` (`String`): Extra data to include, comma-separated, as in `ReaderStreamParams::meta`.
- `fields` (`String`): The fields of each post to return, comma-separated, as in `ReaderStreamParams::fields`. `editorial` has the time a post is shown. The page's `date_range` arrives whichever fields are asked for, and a stream with no posts still answers with a page, not an empty list.

Public documentation: `rest/v1.1/help`, which documents the `rest/v1.1` endpoint only. It declares `after`, `before`, `content_width`, `fields`, `meta`, `number`, `thumb_height`, `thumb_width`, which the library sends.

Declared there, and not sent by the library:

- `format`: (string) Output format. Accepts "json" or "rss". Default: json. Default: 'json'.

### `GET /rest/v1.2/read/search`

A search of Reader posts. Answers signed out.

Request `ReaderPostsRequest::Search`, response `ReaderSearchPostsResponse`.

Parameters the library sends, in the query (`ReaderSearchPostsParams`):

- `q` (`String`, required): What to search for. Without it the API answers 400.
- `number` (`u32`): The number of posts to return: 20 by default, and 20 at most. The API answers 400 to more.
- `page_handle` (`String`): The previous page's `ReaderSearchPostsResponse::next_page_handle`. Send the `sort` that returned it: without it, the API ignores a handle from a search by date.
- `offset` (`u32`): How many results to skip: 199 at most, and the API answers 400 to more. A search by relevance pages this way, and its page handle holds the next offset.
- `sort` (`ReaderSearchSort`): The order of the results. The API's default is `ReaderSearchSort::Relevance`.
- `before` (`WpGmtDateTime`): Only return posts published before this time.
- `after` (`WpGmtDateTime`): Only return posts published after this time.
- `lang` (`String`): The language to search in, such as `fr`.
- `algorithm` (`String`): The name of the algorithm to search with. The API sends back the name it's given, in `ReaderSearchPostsResponse::algorithm` and in the page handle. Another name changed nothing in the results that were looked at.
- `meta` (`String`): Extra data to include, comma-separated, as in `ReaderStreamParams::meta`.
- `fields` (`String`): The fields of each post to return, comma-separated, as in `ReaderStreamParams::fields`. A search with no results still answers with a page, not an empty list.

Public documentation: not listed.

### `GET /rest/v1.3/read/list/$owner/$list/posts`

A Reader list's posts. Answers signed out for a public list, and 404 for a list that doesn't exist.

Request `ReaderPostsRequest::ListPosts`, response `ReaderListPostsResponse`.

Parameters the library sends, in the query (`ReaderListPostsParams`):

- `number` (`u32`): The most posts to return: 10 by default. A page had 197 at most, whatever was asked for, and can have far fewer than were asked for without being the last.
- `page_handle` (`String`): The previous page's `ReaderListPostsResponse::next_page_handle`.
- `page` (`u32`): The page to return, counting from 1. It changed nothing in the responses that were looked at.
- `order` (`ReaderStreamOrder`): The order of the posts. It changed nothing in the responses that were looked at.
- `before` (`WpGmtDateTime`): Only return posts published before this time. It changed nothing in the responses that were looked at.
- `after` (`WpGmtDateTime`): Only return posts published after this time. It changed nothing in the responses that were looked at.

Public documentation: not listed.

### `GET /wpcom/v2/read/streams/discover`

Discover's stream of recommended posts, as cards. Answers signed out.

Request `ReaderPostsRequest::Discover`, response `ReaderDiscoverResponse`.

Parameters the library sends, in the query (`ReaderDiscoverParams`):

- `count` (`u32`): The number of cards to return: 8 by default. The card of recommended sites is one of them. The API returned as many as 120 when asked, and 86 when asked for 150 or more. It answers 500 to `0`.
- `page_handle` (`String`): The previous page's `ReaderDiscoverResponse::next_page_handle`. The API answers 500 to a handle it didn't send.
- `tags` (`Vec<ReaderTagSlug>`, required): Tags to lean the stream toward. With any, the page also ends with a card of tags the user may like, which isn't counted in `count`.
- `sort` (`ReaderDiscoverSort`): How the posts are picked. The API's default is `ReaderDiscoverSort::Popular`.
- `age_based_decay` (`f64`): How much to hold a post's age against it, between 0 and 1. It changed nothing in the responses that were looked at.

Public documentation: not listed.

### `GET /wpcom/v2/read/streams/first-posts`

The first posts of new sites, as cards. Answers signed out.

Request `ReaderPostsRequest::FirstPosts`, response `ReaderFirstPostsResponse`.

Parameters the library sends, in the query (`ReaderFirstPostsParams`):

- `count` (`u32`): The number of cards to ask for: 8 by default. A page has fewer posts than that, often half as many. Asked for 16 or more, the API returns no posts and no handle, and it answers 500 to `0`.
- `page_handle` (`String`): The previous page's `ReaderFirstPostsResponse::next_page_handle`. The API answers a handle it didn't send with the first page.

Public documentation: not listed.

### `GET /wpcom/v2/read/tags/posts`

The posts of several tags, which is Discover's "latest" tab. Answers signed out.

Request `ReaderPostsRequest::TagsPosts`, response `ReaderTagPostsResponse`.

Parameters the library sends, in the query (`ReaderTagsPostsParams`):

- `tags` (`Vec<ReaderTagSlug>`, required): The tags to return posts for. Without any, the API answers 400.
- `number` (`u32`): The number of posts to return. The API returns 10 at most, whatever is asked for.
- `page_handle` (`String`): The previous page's `ReaderTagPostsResponse::next_page_handle`, from a page sorted by date. A handle from a page sorted by popularity returns the first page again.
- `page` (`u32`): The page to return, counting from 1. It works in either order, and it's the only way to page posts sorted by popularity.
- `sort` (`ReaderTagPostsSort`): The order of the posts. The API's default is `ReaderTagPostsSort::Date`.
- `before` (`WpGmtDateTime`): Only return posts published before this time.
- `after` (`WpGmtDateTime`): Only return posts published after this time.

Public documentation: the `wpcom/v2` route index. It declares `after`, `before`, `number`, `page`, `page_handle`, `sort`, `tags`, which the library sends.

Declared there, and not sent by the library:

- `_locale`: (string) Locale of the client.

## Single posts

`wp_api/src/wp_com/endpoint/reader_post_endpoint.rs`

### `GET /rest/v1.2/read/sites/$site/posts/$post_id`

One post of a WordPress.com or Jetpack site, in the shape a stream sends. Answers signed out for a public site. A post that isn't there is a 404 `unknown_post`, and a site that isn't a 404 `unknown_blog`. A private site's post is a 403 `unauthorized` to anyone who can't read the site, and so is a trashed post asked for signed out.

For a trashed post asked for by someone who can edit it, and for a post of a deleted site, the API answers with an HTTP 400 whose `error` is the number 404. That isn't an error code, so it arrives as `WpApiError::UnknownError`.

Request `ReaderPostRequest::SitePost`, response `ReaderPost`.

Parameters the library sends, in the query (`ReaderPostParams`):

- `meta` (`String`): Extra data to include, comma-separated. For a site's post, `site` sends the post's site, which has its `ReaderPost::site_locale`, and `likes` its likes. For a feed's post, `feed` sends the feed, which has an RSS feed's image for `ReaderPost::site_icon_url`. Signed out, the site arrives without its locale.
- `fields` (`String`): The fields of the post to return, comma-separated, such as `ID,site_ID,date`. Without it the post arrives with every field. `ID` has to be among them: a post without one can't be read, and the request fails.

Public documentation: `rest/v1.2/help`. It declares `fields`, `meta`, which the library sends.

### `GET /rest/v1.3/read/feed/$feed/posts/$feed_item`

One post of a feed, in the shape a stream sends. Answers signed out. An item that isn't in the feed is a 404 `no_such_item`, and a feed that isn't there a 404 `no_such_feed`.

A stream can list an item this request can't fetch. The Following stream sends some posts of a Jetpack site as external feed items, and several of those are a 404 `post_not_found` here.

Request `ReaderPostRequest::FeedPost`, response `ReaderPost`.

Parameters the library sends, in the query (`ReaderPostParams`): `meta`, `fields`. They're described under `GET /rest/v1.2/read/sites/$site/posts/$post_id`.

Public documentation: `rest/v1.3/help`. It declares `fields`, `meta`, which the library sends.

### `GET /rest/v1.1/sites/$site/posts/slug:$slug`

A site's post, found by its slug. The site can be an ID or a domain. This is the general posts endpoint, so the post isn't in the shape a stream sends. Answers signed out for a public site. A slug that isn't there is a 404 `unknown_post`, and a private site's post a 403 `unauthorized` to anyone who can't read the site.

Request `ReaderPostRequest::PostBySlug`, response `ReaderPostBySlug`.

Parameters the library sends, in the query (`ReaderPostBySlugParams`):

- `fields` (`String`): The fields of the post to return, comma-separated. `ID,site_ID` is enough to then fetch the post as a `ReaderPost`, and leaves out the post's content. `ID` and `site_ID` have to be among them, or the request fails.

Public documentation: `rest/v1.1/help`. It declares `fields`, which the library sends.

### `GET /rest/v1.2/read/site/$site/post/$post_id/related`

The posts related to a site's post. Answers signed out for a public site. A post that isn't there is a 404 `unknown_post`, and a private site's post a 403 `unauthorized` to anyone who can't read the site.

Request `ReaderPostRequest::RelatedPosts`, response `ReaderRelatedPostsResponse`.

Parameters the library sends, in the query (`ReaderRelatedPostsParams`):

- `size_global` (`u32`): How many posts from other sites to return: 20 at most, or the request fails.
- `size_local` (`u32`): How many posts from the same site to return: 20 at most, or the request fails.
- `lang` (`String`): The language to look for posts in. It showed no effect when this was written: posts from other sites came back in the post's own language whatever was asked for.
- `algorithm` (`String`): The name of the algorithm to pick every post with. The response names it back. A name the API doesn't know still returns posts.
- `algorithm_local` (`String`): The name of the algorithm to pick the same site's posts with.
- `algorithm_global` (`String`): The name of the algorithm to pick other sites' posts with.
- `shuffle_posts_local` (`bool`): Whether to shuffle the same site's posts before taking `size_local` of them. The API's default is not to.
- `shuffle_posts_global` (`bool`): Whether to shuffle other sites' posts before taking `size_global` of them. The API's default is to. Their order differs from one request to the next either way.
- `meta` (`String`): Extra data to include, comma-separated: `site` sends each post's site, which has its `ReaderPost::site_locale`.
- `fields` (`String`): The fields of each post to return, comma-separated, such as `ID,site_ID,railcar`. `ID` has to be among them. A post without one can't be read, and is listed in `ReaderRelatedPostsResponse::unreadable_posts`.

Public documentation: not listed.

### `GET /wpcom/v2/mobile/resolve-reader-url`

The site and post IDs of the post at an address. Needs a signed-in user: signed out it's a 401 `rest_forbidden`. An address that isn't a post's is a 404 `post_not_found`, and one without a path a 400 `rest_invalid_param`. The API also answers a few addresses with a 500 and no body, such as one on a `wordpress.com` subdomain that doesn't exist.

Request `ReaderPostRequest::ResolveUrl`, response `ReaderResolvedUrl`.

Parameters the library sends, in the query (`ReaderResolveUrlParams`):

- `url` (`String`, required): The post's address, such as `https://example.com/2026/11/19/hello-world/`. It needs a scheme and a path: the API refuses `example.com`, and `https://example.com/` too. `http` or `https`, the host in any case, a query and a fragment all resolve the same. The address can be on the site's own domain. A short link, such as `https://wp.me/...`, and the `?p=123` form don't resolve.

Public documentation: the `wpcom/v2` route index. It declares `url`, which the library sends.

## Sites and feeds

`wp_api/src/wp_com/endpoint/reader_sites_endpoint.rs`

### `GET /rest/v1.2/read/sites/$site`

A WordPress.com or Jetpack site's Reader info. The site can be an ID or a domain. Answers signed out for a public site, with nothing about the user. A site that isn't there is a 404 `unknown_blog`, a deleted one a 410 `site_gone`, and a private one the user can't read a 403 `unauthorized`.

Request `ReaderSitesRequest::Site`, response `ReaderSite`.

Parameters the library sends, in the query (`ReaderSiteParams`):

- `fields` (`String`): The fields of the site to return, comma-separated, such as `ID,name,feed_ID,is_following`. Without it the site arrives with every field, which for a site the user can manage is about 75 kB. `ID` has to be among them, or the request fails.
- `options` (`String`): Which of the site's options to return, comma-separated, such as `is_mapped_domain,unmapped_url`. Only someone who can manage the site is sent any options, and `ReaderSite` keeps none of them, so all this does is make that response smaller.

Public documentation: not listed.

### `GET /rest/v1.1/read/feed/$feed`

A feed's Reader info. Answers signed out for a public feed, with nothing about the user. A feed that isn't there is a 404 `no_such_feed`, and the feed of a private site the user can't read a 403 `unauthorized`.

Request `ReaderSitesRequest::Feed`, response `ReaderFeed`.

The library sends no parameters.

Public documentation: `rest/v1.1/help`.

### `GET /rest/v1.1/read/feed`

Searches sites and feeds for some text, or finds the feeds at an address. Answers signed out. With neither it's a 400 `missing_param`, and an address with no feed is a 404 `no_such_feed`.

Request `ReaderSitesRequest::SearchFeeds`, response `ReaderFeedSearchResponse`.

Parameters the library sends, in the query (`ReaderFeedSearchParams`):

- `q` (`String`): The text to search sites and feeds for. Text that looks like an address, such as `example.com`, is looked up the way `url` is.
- `url` (`String`): The address of a site or a feed, to find the feeds there, such as `example.com` or `https://example.com/feed/`. An address with no feed is a `no_such_feed` error.
- `number` (`u32`): How many results to return: 10 by default and 20 at most, or the request fails. A lookup by address ignores it.
- `offset` (`u32`): How many results to skip: 199 at most, or the request fails.
- `exclude_followed` (`bool`): Whether to leave out the sites the user follows. The API's default is to. It makes no difference signed out.
- `sort` (`ReaderFeedSearchSort`): The order of the results. The API's default is `ReaderFeedSearchSort::Relevance`.
- `page_handle` (`String`): The previous page's `ReaderFeedSearchResponse::next_page_handle`, when sorting by `ReaderFeedSearchSort::LastUpdated`.
- `algorithm` (`String`): The name of the algorithm to search with. The response names it back. A name the API doesn't know still returns results.
- `meta` (`String`): Extra data to include, comma-separated: `site` sends each result's `ReaderFeedSearchResult::site` and `feed` its `ReaderFeedSearchResult::feed`.

Public documentation: `rest/v1.1/help`. It declares `algorithm`, `exclude_followed`, `meta`, `number`, `offset`, `page_handle`, `q`, `sort`, `url`, which the library sends.

## Tags and the menu

`wp_api/src/wp_com/endpoint/reader_tags_endpoint.rs`

### `GET /rest/v1.3/read/menu`

The Reader menu: its streams, the tags the user follows, and tags to suggest. Answers signed out, with Discover and the suggested tags only.

Request `ReaderTagsRequest::Menu`, response `ReaderMenu`.

The library sends no parameters.

Public documentation: `rest/v1.3/help`.

### `GET /rest/v1.2/read/tags/$tag`

A tag, by its slug. Answers signed out. A slug that names no tag is a 404 `invalid_tag`.

Request `ReaderTagsRequest::Tag`, response `ReaderTagResponse`.

The library sends no parameters.

Public documentation: `rest/v1.1/help`, which documents the `rest/v1.1` endpoint only.

### `GET /wpcom/v2/read/interests`

The tags a new reader picks from. Answers signed out.

Request `ReaderTagsRequest::Interests`, response `ReaderInterestsResponse`.

The library sends no parameters.

Public documentation: the `wpcom/v2` route index.

### `POST /rest/v1.2/read/tags/mine/new`

Follows one or more tags, and answers with every tag the user now follows. For a tag the user already follows it's still a 200, with nothing in `added_tags`.

Request `ReaderTagsRequest::FollowTags`, response `ReaderFollowTagsResponse`.

Parameters the library sends, in the body (`ReaderFollowTagsParams`):

- `tags` (`Vec<ReaderTagSlug>`, required): The slugs of the tags to follow, sent as a list, as the iOS app sends them. The API also takes one comma-separated string, which the Android app sends. One tag was tried: an empty list wasn't, and neither was a slug that isn't ASCII.

Public documentation: not listed.

### `POST /rest/v1.1/read/tags/$tag/mine/delete`

Unfollows a tag, by its slug, and answers with every tag the user still follows. It takes no parameters. For a tag the user doesn't follow it's a 400 `not_subscribed`.

Request `ReaderTagsRequest::UnfollowTag`, response `ReaderUnfollowTagResponse`.

The library sends no parameters.

Public documentation: `rest/v1.1/help`.

## Users

`wp_api/src/wp_com/endpoint/reader_users_endpoint.rs`

### `GET /rest/v1.1/users/$user`

A user's public profile, by login or by ID. Needs a signed-in user: signed out it's a 403 `authorization_required`. A user who isn't there is a 404 `unknown_user`.

Request `ReaderUsersRequest::Profile`, response `ReaderUserProfile`.

The library sends no parameters.

Public documentation: `rest/v1.1/help`.

## Saved posts

`wp_api/src/wp_com/endpoint/reader_saved_endpoint.rs`

### `GET /rest/v1.2/read/saved`

The posts the user saved, newest save first, a page at a time. Needs a token.

Request `ReaderSavedRequest::ListPosts`, response `ReaderSavedPostsResponse`.

Parameters the library sends, in the query (`ReaderSavedPostsParams`):

- `page_handle` (`String`): The previous page's `ReaderSavedPostsResponse::next_page_handle`.
- `number` (`u32`): The number of posts to return: 10 by default, and at most 40.
- `after` (`WpGmtDateTime`): Only return posts saved after this time.
- `before` (`WpGmtDateTime`): Only return posts saved before this time. Ignored when `page_handle` is set.
- `meta` (`String`): Extra data to include, comma-separated. `site,feed` gives every post a `ReaderPost::site_icon_url` that has one: `feed` adds an RSS feed's image.
- `orderBy` (`ReaderSavedOrderBy`): What the list is ordered by: when each post was saved, unless this says otherwise. Send the same value with a `page_handle` as the request that returned it.
- `order` (`ReaderStreamOrder`): The order of the list: newest first, unless this says otherwise. Send the same value with a `page_handle` as the request that returned it. The API reads it only in capitals, `ASC` and `DESC`, which is how it's sent: it takes `asc` for the default.

Public documentation: not listed.

### `GET /wpcom/v2/read/saved/keys`

The key of every post the user saved, newest first, up to 1,000. Needs a token.

Request `ReaderSavedRequest::ListKeys`, response `ReaderSavedKeysResponse`.

The library sends no parameters.

Public documentation: the `wpcom/v2` route index.

### `POST /wpcom/v2/read/saved/new`

Saves a post, an RSS post, or a page by its address. Needs a token. A user has at most 1,000 saves: one more is refused with `saved_limit_reached`.

Request `ReaderSavedRequest::Save`, response `ReaderSavePostResponse`.

Parameters the library sends, in the body (`ReaderSavePostParams`):

- `blog_id` (`WpComSiteId`): A WordPress.com post.
- `post_id` (`PostId`): A WordPress.com post.
- `feed_item_id` (`ReaderFeedItemId`): An RSS post.
- `url` (`String`): A page that isn't a Reader post. Saving a URL can still return one of the other keys: a URL already saved keeps its key, and the URL of a WordPress.com post resolves to the post.
- `source` (`ReaderSavedSource`, required): The app making the save, which is recorded with it.
- `title` (`String`): What to show for a page saved by URL, which the server doesn't fetch. Kept only when the URL isn't a WordPress.com post.
- `excerpt` (`String`): What to show for a page saved by URL, which the server doesn't fetch. Kept only when the URL isn't a WordPress.com post.
- `image_url` (`String`): What to show for a page saved by URL, which the server doesn't fetch. Kept only when the URL isn't a WordPress.com post.
- `author` (`String`): What to show for a page saved by URL, which the server doesn't fetch. Kept only when the URL isn't a WordPress.com post.
- `site_name` (`String`): What to show for a page saved by URL, which the server doesn't fetch. Kept only when the URL isn't a WordPress.com post.

Public documentation: the `wpcom/v2` route index. It declares `author`, `blog_id`, `excerpt`, `feed_item_id`, `image_url`, `post_id`, `site_name`, `source`, `title`, `url`, which the library sends.

Declared there, and not sent by the library:

- `feed_id`: (integer)
- `site_icon_url`: (string)

### `POST /wpcom/v2/read/saved/delete`

Unsaves what a key names. Needs a token.

Request `ReaderSavedRequest::Unsave`, response `ReaderUnsavePostResponse`.

Parameters the library sends, in the body (`ReaderUnsavePostParams`):

- `blog_id` (`WpComSiteId`): A WordPress.com post.
- `post_id` (`PostId`): A WordPress.com post.
- `feed_item_id` (`ReaderFeedItemId`): An RSS post.
- `url` (`String`): A page that isn't a Reader post. Saving a URL can still return one of the other keys: a URL already saved keeps its key, and the URL of a WordPress.com post resolves to the post.
- `source` (`ReaderSavedSource`, required): The app removing the save.

Public documentation: the `wpcom/v2` route index. It declares `blog_id`, `feed_item_id`, `post_id`, `source`, `url`, which the library sends.

Declared there, and not sent by the library:

- `feed_id`: (integer)

### `POST /wpcom/v2/read/saved/bulk`

Saves up to 100 posts or RSS posts in one request, and answers with one result a key. Needs a token.

Request `ReaderSavedRequest::BulkSave`, response `ReaderSavedBulkResponse`.

Parameters the library sends, in the body (`ReaderSavedBulkParams`):

- `keys` (`Vec<ReaderSavedBulkKey>`, required): The keys to save, newest first: between 1 and 100, or the whole request is rejected. If they would take the user past the 1,000 save cap, the newest are the ones saved.
- `source` (`ReaderSavedSource`, required): The app making the saves, which is recorded with each.

Public documentation: the `wpcom/v2` route index. It declares `keys`, `source`, which the library sends.

## Seen status

`wp_api/src/wp_com/endpoint/reader_seen_endpoint.rs`

### `POST /wpcom/v2/seen-posts/seen/blog/new`

Marks posts of a WordPress.com or Jetpack site as seen.

Request `ReaderSeenRequest::SitePostsSeen`, response `ReaderSeenResponse`.

Parameters the library sends, in the body (`ReaderSeenSitePostsParams`):

- `blog_id` (`WpComSiteId`, required): The site the posts are on.
- `post_ids` (`Vec<PostId>`, required): At least one. The API refuses an empty list with `rest_invalid_param`.
- `source` (`ReaderSeenSource`, required): The app marking the posts, which is recorded with the mark.

Public documentation: the `wpcom/v2` route index. It declares `blog_id`, `post_ids`, `source`, which the library sends.

### `POST /wpcom/v2/seen-posts/seen/blog/delete`

Marks posts of a WordPress.com or Jetpack site as unseen.

Request `ReaderSeenRequest::SitePostsUnseen`, response `ReaderSeenResponse`.

Parameters the library sends, in the body (`ReaderSeenSitePostsParams`): `blog_id`, `post_ids`, `source`. They're described under `POST /wpcom/v2/seen-posts/seen/blog/new`.

Public documentation: the `wpcom/v2` route index. It declares `blog_id`, `post_ids`, `source`, which the library sends.

### `POST /wpcom/v2/seen-posts/seen/new`

Marks posts of an RSS feed as seen.

Request `ReaderSeenRequest::FeedItemsSeen`, response `ReaderSeenResponse`.

Parameters the library sends, in the body (`ReaderSeenFeedItemsParams`):

- `feed_id` (`ReaderFeedId`, required): The feed the posts are in.
- `feed_item_ids` (`Vec<ReaderFeedItemId>`, required): At least one.
- `source` (`ReaderSeenSource`, required): The app marking the posts, which is recorded with the mark.

Public documentation: the `wpcom/v2` route index. It declares `feed_id`, `feed_item_ids`, `source`, which the library sends.

### `POST /wpcom/v2/seen-posts/seen/delete`

Marks posts of an RSS feed as unseen.

Request `ReaderSeenRequest::FeedItemsUnseen`, response `ReaderSeenResponse`.

Parameters the library sends, in the body (`ReaderSeenFeedItemsParams`): `feed_id`, `feed_item_ids`, `source`. They're described under `POST /wpcom/v2/seen-posts/seen/new`.

Public documentation: the `wpcom/v2` route index. It declares `feed_id`, `feed_item_ids`, `source`, which the library sends.

## Likes

`wp_api/src/wp_com/endpoint/reader_likes_endpoint.rs`

### `GET /rest/v1.2/sites/$site/posts/$post_id/likes`

The users who liked a post, newest like first, paged by the date of the last like. Answers signed out for a public site; a private site's is then a 403 `unauthorized`. A post that doesn't exist is a 404 `unknown_post`.

Request `ReaderLikesRequest::PostLikes`, response `ReaderLikesResponse`.

Parameters the library sends, in the query (`ReaderLikesParams`):

- `number` (`u32`): The number of likes to return: 20 by default, and 90 at most, whatever is asked for.
- `before` (`WpGmtDateTime`): Only return likes from before this time, to the second.
- `exclude` (`Vec<WpComUserId>`, required): Users whose likes to leave out. `ReaderLikesResponse::found` still counts them.

Public documentation: `rest/v1.2/help`. It declares `before`, `exclude`, `number`, which the library sends.

### `GET /rest/v1.2/sites/$site/comments/$comment_id/likes`

The users who liked a comment, with the same parameters and paging as a post's likes. Answers signed out for a public site. A comment that doesn't exist is a 404 `unknown_comment`.

Request `ReaderLikesRequest::CommentLikes`, response `ReaderLikesResponse`.

Parameters the library sends, in the query (`ReaderLikesParams`): `number`, `before`, `exclude`. They're described under `GET /rest/v1.2/sites/$site/posts/$post_id/likes`.

Public documentation: `rest/v1.2/help`. It declares `before`, `exclude`, `number`, which the library sends.

### `POST /rest/v1.1/sites/$site/posts/$post_id/likes/new`

Likes a post, for the user. It takes no parameters. In trials the API refused every like with a 400 `like_failed` and an empty message, for a reason that isn't known, so the answer to a like that works wasn't seen.

Request `ReaderLikesRequest::LikePost`, response `ReaderPostLikeResponse`.

The library sends no parameters.

Public documentation: `rest/v1.1/help`.

### `POST /rest/v1.1/sites/$site/posts/$post_id/likes/mine/delete`

Removes the user's like from a post. It takes no parameters. For a post the user doesn't like it's still a 200, with `success: false`.

Request `ReaderLikesRequest::UnlikePost`, response `ReaderPostLikeResponse`.

The library sends no parameters.

Public documentation: `rest/v1.1/help`.

### `POST /rest/v1.1/sites/$site/comments/$comment_id/likes/new`

Likes a comment, for the user. It takes no parameters. In trials the API refused the like with a 400 `comment_like_failed` and an empty message, for a reason that isn't known, so the answer to a like that works wasn't seen.

Request `ReaderLikesRequest::LikeComment`, response `ReaderCommentLikeResponse`.

The library sends no parameters.

Public documentation: `rest/v1.1/help`.

### `POST /rest/v1.1/sites/$site/comments/$comment_id/likes/mine/delete`

Removes the user's like from a comment. It takes no parameters. For a comment the user doesn't like it's still a 200, with `success: false`.

Request `ReaderLikesRequest::UnlikeComment`, response `ReaderCommentLikeResponse`.

The library sends no parameters.

Public documentation: `rest/v1.1/help`.

## Subscriptions

`wp_api/src/wp_com/endpoint/reader_subscriptions_endpoint.rs`

### `GET /rest/v1.2/read/following/mine`

The sites and feeds the user is subscribed to, by page number. Needs a token.

Request `ReaderSubscriptionsRequest::List`, response `ReaderSubscriptionsResponse`.

Parameters the library sends, in the query (`ReaderSubscriptionsParams`):

- `page` (`u32`): The page to return, counting from 1. `0` returns the first page.
- `number` (`u32`): The number of subscriptions a page covers. Ask for 100 at most, which is what the apps and the web Reader ask for. Without it, or with `0`, the API returned every subscription of an account that has 37.
- `q` (`String`): Text to search the subscriptions for. In trials every search came back empty, even for the whole name or address of a subscription, so don't rely on it. An empty `q` is ignored.
- `filter` (`String`): Only return some subscriptions. `a8c` returns the ones to Automattic's sites: it drops the others from each page after the page is cut, so a page of 4 can come back with 2. No other value tried changed the list.
- `meta` (`String`): Extra data to include, comma-separated. `site,feed`, which the apps send, makes the API add each subscription's whole site and feed, which `ReaderSubscription` doesn't keep.

Public documentation: `rest/v1.2/help`. It declares `filter`, `meta`, `number`, `page`, `q`, which the library sends.

### `GET /rest/v1.1/sites/$site/follows/mine`

Whether the user follows a site. Needs a token. It takes a site's ID, not a feed's: a feed's ID is read as some other site's.

Request `ReaderSubscriptionsRequest::IsFollowingSite`, response `ReaderSiteFollowStatus`.

The library sends no parameters.

Public documentation: `rest/v1.1/help`.

### `POST /rest/v1.1/sites/$site/follows/new`

Follows a WordPress.com or Jetpack site, by its ID, and answers as an unfollow does. For a site the user follows already it's a 400 `already_subscribed`.

When this was first tried, the API refused every follow with a 400 `follow_failed` and an empty message. A day later it took them, with nothing changed on this side.

The API declares a `source` for it, which says where the follow came from, and reads it from the address only: the apps add `?source=android` or `?source=ios`. A POST here sends its parameters in the body, so no `source` is sent. A follow by address takes one.

Request `ReaderSubscriptionsRequest::FollowSite`, response `ReaderFollowSiteResponse`.

The library sends no parameters.

Public documentation: `rest/v1.1/help`.

Declared there, and not sent by the library:

- `source`: (string) The source of the follow

### `POST /rest/v1.1/sites/$site/follows/mine/delete`

Unfollows a site, by its ID. It takes no parameters. For a site the user doesn't follow it's a 400 `are_not_subscribed`.

Request `ReaderSubscriptionsRequest::UnfollowSite`, response `ReaderFollowSiteResponse`.

The library sends no parameters.

Public documentation: `rest/v1.1/help`.

### `POST /rest/v1.1/read/following/mine/new`

Follows a site or a feed, by its address, and answers with the new subscription. An address with no feed is still a 200, with `subscribed: false` and `info: "invalid_feed"`.

Request `ReaderSubscriptionsRequest::FollowUrl`, response `ReaderFollowUrlResponse`.

Parameters the library sends, in the body (`ReaderFollowUrlParams`):

- `url` (`String`, required): The address of the site or feed to follow.
- `source` (`String`): Where the follow came from. The apps send `android` and `ios`. Other text is taken too, and none tried changed the answer.

Public documentation: `rest/v1.1/help`. It declares `source`, `url`, which the library sends.

### `POST /rest/v1.1/read/following/mine/delete`

Unfollows a site or a feed, by its subscription's ID or by the address that was followed.

Request `ReaderSubscriptionsRequest::UnfollowUrl`, response `ReaderUnfollowUrlResponse`.

Parameters the library sends, in the body (`ReaderUnfollowUrlParams`):

- `sub_id` (`ReaderSubscriptionId`): The subscription to remove: a `ReaderSubscription::id`. The web Reader sends it rather than `url` whenever it has one.
- `url` (`String`): The address that was followed, for when the subscription's ID isn't known.
- `blog_id` (`WpComSiteId`): The site to unsubscribe from.
- `email_id` (`String`): An email's tracking ID, for an unfollow made from an email.

Public documentation: `rest/v1.1/help`. It declares `blog_id`, `email_id`, `sub_id`, `url`, which the library sends.

### `POST /rest/v1.2/read/site/$site/post_email_subscriptions/new`

Starts emailing the user a site's new posts.

Request `ReaderSubscriptionsRequest::SubscribeToPostEmails`, response `ReaderPostEmailSubscriptionResponse`.

Parameters the library sends, in the body (`ReaderPostEmailSubscriptionParams`):

- `delivery_frequency` (`ReaderSubscriptionEmailFrequency`): How often to email the posts, for a subscription that's starting. The web Reader sends it when it starts one. A subscription started without it was `ReaderSubscriptionEmailFrequency::Instantly`.
- `track_source` (`String`): Where the change was made, which the API records.
- `email_id` (`String`): An email's tracking ID, for a change made from an email.

Public documentation: `rest/v1.2/help`. It declares `delivery_frequency`, `track_source`, which the library sends. It doesn't declare `email_id`.

### `POST /rest/v1.2/read/site/$site/post_email_subscriptions/delete`

Stops emailing the user a site's new posts.

Request `ReaderSubscriptionsRequest::UnsubscribeFromPostEmails`, response `ReaderPostEmailSubscriptionResponse`.

Parameters the library sends, in the body (`ReaderPostEmailSubscriptionParams`): `delivery_frequency`, `track_source`, `email_id`. They're described under `POST /rest/v1.2/read/site/$site/post_email_subscriptions/new`.

Public documentation: `rest/v1.2/help`. It declares `email_id`, which the library sends. It doesn't declare `delivery_frequency`, `track_source`.

### `POST /rest/v1.2/read/site/$site/post_email_subscriptions/update`

Changes how often a site's new posts are emailed. A frequency the API doesn't know is still a 200 with `success: true`, and sets the emails to `instantly`.

Request `ReaderSubscriptionsRequest::UpdatePostEmails`, response `ReaderPostEmailSubscriptionResponse`.

Parameters the library sends, in the body (`ReaderPostEmailFrequencyParams`):

- `delivery_frequency` (`ReaderSubscriptionEmailFrequency`, required): How often to email the posts. Send `Instantly`, `Daily` or `Weekly`: the API takes anything else as `Instantly`, without an error.
- `track_source` (`String`): Where the change was made, which the API records. It wasn't tried.

Public documentation: `rest/v1.2/help`. It declares `delivery_frequency`, `track_source`, which the library sends.

### `POST /rest/v1.2/read/site/$site/comment_email_subscriptions/new`

Starts emailing the user the new comments on a site's posts.

The API declares a `post_id` for this and for stopping them, which it reads from the address only. A POST here sends its parameters in the body, so no `post_id` is sent, and both act on the whole site.

Request `ReaderSubscriptionsRequest::SubscribeToCommentEmails`, response `ReaderCommentEmailSubscriptionResponse`.

The library sends no parameters.

Public documentation: `rest/v1.2/help`.

Declared there, and not sent by the library:

- `post_id`: (int) Post ID (optional)

### `POST /rest/v1.2/read/site/$site/comment_email_subscriptions/delete`

Stops emailing the user the new comments on a site's posts.

Request `ReaderSubscriptionsRequest::UnsubscribeFromCommentEmails`, response `ReaderCommentEmailSubscriptionResponse`.

The library sends no parameters.

Public documentation: `rest/v1.2/help`.

Declared there, and not sent by the library:

- `post_id`: (int) Post ID (optional)

### `POST /wpcom/v2/read/sites/$site/notification-subscriptions/new`

Turns on notifications of a site's new posts. It takes no parameters.

Request `ReaderSubscriptionsRequest::SubscribeToNotifications`, response `ReaderNotificationSubscriptionResponse`.

The library sends no parameters.

Public documentation: the `wpcom/v2` route index.

### `POST /wpcom/v2/read/sites/$site/notification-subscriptions/delete`

Turns off notifications of a site's new posts. It takes no parameters.

Request `ReaderSubscriptionsRequest::UnsubscribeFromNotifications`, response `ReaderNotificationSubscriptionResponse`.

The library sends no parameters.

Public documentation: the `wpcom/v2` route index.

## Blocked sites

`wp_api/src/wp_com/endpoint/reader_blocks_endpoint.rs`

### `GET /wpcom/v2/me/blocks/sites`

The sites the user blocked.

Request `ReaderBlocksRequest::ListSites`, response `ReaderBlockedSitesResponse`.

Parameters the library sends, in the query (`ReaderBlockedSitesParams`):

- `page` (`u32`): The page to return, from 1.
- `per_page` (`u32`): How many sites a page has. The API's default is 100.

Public documentation: the `wpcom/v2` route index. It declares `page`, `per_page`, which the library sends.

### `POST /rest/v1.1/me/block/sites/$site/new`

Blocks a site.

Request `ReaderBlocksRequest::BlockSite`, response `ReaderBlockSiteResponse`.

The library sends no parameters.

Public documentation: not listed.

### `POST /rest/v1.1/me/block/sites/$site/delete`

Unblocks a site.

Request `ReaderBlocksRequest::UnblockSite`, response `ReaderBlockSiteResponse`.

The library sends no parameters.

Public documentation: not listed.

## Comments

`wp_api/src/wp_com/endpoint/reader_comments_endpoint.rs`

### `GET /rest/v1.1/sites/$site/posts/$post_id/replies`

A post's comments, by offset, page number or date. Answers signed out for a public site; a private site's is then a 403 `unauthorized`. A post that doesn't exist is a 404 `unknown_post`.

Request `ReaderCommentsRequest::List`, response `ReaderCommentsResponse`.

Parameters the library sends, in the query (`ReaderCommentsParams`):

- `number` (`u32`): The number of comments to return: 20 by default, and 100 at most. More is a 400 `invalid_number`. With `hierarchical` it counts top-level comments.
- `offset` (`u32`): The number of comments to skip. Ignored with `hierarchical`.
- `page` (`u32`): The page to return, counting from 1. It wins over `offset`.
- `order` (`ReaderCommentsOrder`): The order of the comments. The API's default is `ReaderCommentsOrder::Desc`.
- `hierarchical` (`bool`): Return whole threads: each top-level comment is followed by every reply under it, at any depth, and a page holds `number` top-level comments with all of theirs. The comments are still a flat list. Only `page` moves through it: `offset`, `before` and `after` are ignored. A page past the last one isn't empty: the API sends every comment again. So stop at a page with fewer than `number` top-level comments, or once the pages hold `ReaderCommentsResponse::found` comments.
- `after` (`WpGmtDateTime`): Only return comments from this time or later. Ignored with `hierarchical`.
- `before` (`WpGmtDateTime`): Only return comments from this time or earlier. Ignored with `hierarchical`.
- `type` (`ReaderCommentsTypeFilter`): The kinds of comment to return. `ReaderCommentsResponse::found` counts only those.
- `status` (`ReaderCommentsStatusFilter`): The moderation statuses to return. Signed out, anything but approved comments is a 403 `unauthorized`.
- `author_wpcom_data` (`bool`): Add `ReaderCommentAuthor::wpcom_id` and `ReaderCommentAuthor::wpcom_login` to each author with a WordPress.com account. On a WordPress.com site they repeat the author's ID and login.
- `force` (`String`): `wpcom` reads a Jetpack site's comments from WordPress.com's copy of them, as the apps and the web Reader do. Without it a Jetpack site's comments have no `ReaderComment::like_count` or `ReaderComment::i_like`. It changes nothing for a WordPress.com site.
- `fields` (`String`): The fields of each comment to return, comma-separated, such as `ID,date,parent`. `ID` has to be among them: a comment without one can't be read, and is listed in `ReaderCommentsResponse::unreadable_comments`.

Public documentation: `rest/v1.1/help`. It declares `after`, `author_wpcom_data`, `before`, `fields`, `hierarchical`, `number`, `offset`, `order`, `page`, `status`, `type`, which the library sends. It doesn't declare `force`.

### `GET /rest/v1.1/sites/$site/posts/$post_id/subscribers/mine`

Whether the user is subscribed to a post's comments. Needs a token: signed out it's a 403 `authorization_required`.

Request `ReaderCommentsRequest::Subscription`, response `ReaderPostCommentsSubscription`.

The library sends no parameters.

Public documentation: `rest/v1.1/help`.

### `POST /rest/v1.1/sites/$site/posts/$post_id/replies/new`

Adds a top-level comment to a post, and answers with it. A comment with nothing in it is a 400 `invalid_input`.

Request `ReaderCommentsRequest::NewComment`, response `ReaderComment`.

Parameters the library sends, in the body (`ReaderNewCommentParams`):

- `content` (`String`, required): What the comment says. The answer has it as written in `ReaderComment::raw_content`, and made into HTML in `ReaderComment::content`. An empty one is a 400 `invalid_input`.

Public documentation: `rest/v1.1/help`. It declares `content`, which the library sends.

### `POST /rest/v1.1/sites/$site/comments/$comment_id/replies/new`

Adds a reply to a comment, and answers with it.

Request `ReaderCommentsRequest::NewReply`, response `ReaderComment`.

Parameters the library sends, in the body (`ReaderNewCommentParams`): `content`. They're described under `POST /rest/v1.1/sites/$site/posts/$post_id/replies/new`.

Public documentation: `rest/v1.1/help`. It declares `content`, which the library sends.

### `POST /rest/v1.1/sites/$site/posts/$post_id/subscribers/new`

Subscribes the user to a post's comments.

Request `ReaderCommentsRequest::Subscribe`, response `ReaderPostCommentsSubscriptionResponse`.

Parameters the library sends, in the body (`ReaderSubscribeToCommentsParams`):

- `receive_notifications` (`bool`): Whether new comments should send the user a notification as well. It wasn't tried, and the apps don't send it: a subscription made without it had notifications off.

Public documentation: `rest/v1.1/help`. It declares `receive_notifications`, which the library sends.

### `POST /rest/v1.1/sites/$site/posts/$post_id/subscribers/mine/delete`

Unsubscribes the user from a post's comments. It takes no parameters. With nothing to remove it's still a 200 with `success: true`.

Request `ReaderCommentsRequest::Unsubscribe`, response `ReaderPostCommentsSubscriptionResponse`.

The library sends no parameters.

Public documentation: `rest/v1.1/help`.

### `POST /rest/v1.1/sites/$site/posts/$post_id/subscribers/mine/update`

Turns the notifications of the user's subscription to a post's comments on or off, and answers with the subscription.

Request `ReaderCommentsRequest::UpdateSubscription`, response `ReaderPostCommentsSubscription`.

Parameters the library sends, in the body (`ReaderCommentsNotificationsParams`):

- `receive_notifications` (`bool`, required): Whether new comments should send the user a notification. The answer calls it `receives_notifications`.

Public documentation: `rest/v1.1/help`. It declares `receive_notifications`, which the library sends.

## The page-view pixel

### `GET https://pixel.wp.com/g.gif`

Counts a view of a post in its site's stats. It isn't an API request: it goes to another host and carries no token. `wp_api/src/wp_com/reader_page_views.rs` builds it, and says which `User-Agent` a view is counted with.

Parameters the library sends, in the query: `v=wpcom`, `reader=1`, `blog` (the site's ID), `post`, `host` (the site's host), `ref=https://wordpress.com/`, `t` (a number that differs each time), and `priv=1` for a private site.

Public documentation: not listed.
