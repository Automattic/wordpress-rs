# WordPress.com REST API Checklist

Tracks which WordPress.com REST API endpoints have been ported to wordpress-rs.

<!-- Format: - [ ] `METHOD /path` -->

## Posts

- [ ] `GET /rest/v1.1/sites/$site/posts` — list/search posts
- [ ] `GET /rest/v1.1/sites/$site/posts/$post_id` — fetch single post
- [ ] `GET /rest/v1.1/sites/$site/posts/$post_id` — fetch post status (fields=status)
- [ ] `GET /rest/v1.2/sites/$site/posts/$post_id/likes` — fetch post likes
- [ ] `POST /rest/v1.2/sites/$site/posts/new` — create new post
- [ ] `POST /rest/v1.2/sites/$site/posts/$post_id` — update existing post
- [ ] `POST /rest/v1.1/sites/$site/posts/$post_id/autosave` — autosave post revision
- [ ] `POST /rest/v1.1/sites/$site/posts/$post_id/delete` — delete/trash post
- [ ] `POST /rest/v1.1/sites/$site/posts/$post_id/restore` — restore trashed post
- [ ] `GET /rest/v1.1/sites/$site/posts/$post_id/autosave` — retrieve autosave
- [ ] `GET /rest/v1.1/sites/$site/post-types` — list post types

## Revisions

- [ ] `GET /rest/v1.1/sites/$site/post/$post_id/diffs` — fetch post revision diffs
- [ ] `GET /rest/v1.1/sites/$site/page/$post_id/diffs` — fetch page revision diffs

## Comments

- [ ] `GET /rest/v1.1/sites/$site/comments` — list comments
- [ ] `GET /rest/v1.1/sites/$site/comments/$comment_id` — fetch single comment
- [ ] `GET /rest/v1.2/sites/$site/comments/$comment_id/likes` — fetch comment likes
- [ ] `POST /rest/v1.1/sites/$site/comments/$comment_id` — update comment
- [ ] `POST /rest/v1.1/sites/$site/comments/$comment_id/delete` — delete comment
- [ ] `POST /rest/v1.1/sites/$site/comments/$comment_id/replies/new` — reply to comment
- [ ] `POST /rest/v1.1/sites/$site/posts/$post_id/replies/new` — new top-level comment
- [ ] `POST /rest/v1.1/sites/$site/comments/$comment_id/likes/new` — like comment
- [ ] `POST /rest/v1.1/sites/$site/comments/$comment_id/likes/mine/delete` — unlike comment

## Taxonomies

- [ ] `GET /rest/v1.1/sites/$site/taxonomies/$taxonomy/terms` — list terms
- [ ] `GET /rest/v1.1/sites/$site/taxonomies/$taxonomy/terms/$slug` — fetch single term
- [ ] `POST /rest/v1.1/sites/$site/taxonomies/$taxonomy/terms/new` — create term
- [ ] `POST /rest/v1.1/sites/$site/taxonomies/$taxonomy/terms/$slug` — update term
- [ ] `POST /rest/v1.1/sites/$site/taxonomies/$taxonomy/terms/$slug/delete` — delete term
- [ ] `GET /rest/v1.1/sites/$site/categories` — list categories (iOS path)
- [ ] `POST /rest/v1.1/sites/$site/categories/new` — create category (iOS path)
- [ ] `GET /rest/v1.1/sites/$site/tags` — list tags (iOS path)
- [ ] `POST /rest/v1.1/sites/$site/tags/new` — create tag (iOS path)
- [ ] `POST /rest/v1.1/sites/$site/tags/slug:$slug` — update tag (iOS path)
- [ ] `POST /rest/v1.1/sites/$site/tags/slug:$slug/delete` — delete tag (iOS path)

## Blogging Prompts

- [ ] `GET /wpcom/v3/sites/$site/blogging-prompts` — fetch blogging prompts (Android v3)
- [ ] `GET /wpcom/v2/sites/$site/blogging-prompts` — fetch blogging prompts (iOS v2)
- [ ] `GET /wpcom/v2/sites/$site/blogging-prompts/settings` — fetch prompts settings
- [ ] `POST /wpcom/v2/sites/$site/blogging-prompts/settings` — update prompts settings

## Stats — Insights

- [x] `GET /rest/v1.1/sites/$site/stats` — all-time stats (visitors, views, posts, best day)
- [ ] `GET /rest/v1.1/sites/$site/stats/comments` — top commenters and most-commented posts
- [ ] `GET /rest/v1.1/sites/$site/stats/followers` — site followers (filterable by type)
- [ ] `GET /rest/v1.1/sites/$site/stats/post/$post_id` — per-post view stats
- [x] `GET /rest/v1.1/sites/$site/stats/insights` — most popular day/hour, yearly aggregates
- [ ] `GET /rest/v1.1/sites/$site/stats/streak` — posting activity/streak data
- [ ] `GET /rest/v1.1/sites/$site/stats/publicize` — social media follower counts
- [ ] `GET /rest/v1.1/sites/$site/stats/summary` — total likes, comments, followers
- [x] `GET /rest/v1.1/sites/$site/stats/tags` — top tags/categories by views

## Stats — Subscribers

- [x] `GET /rest/v1.1/sites/$site/stats/emails/summary` — email newsletter stats
- [x] `GET /rest/v1.1/sites/$site/stats/subscribers` — subscriber count over time

## Stats — Time-Based

- [x] `GET /rest/v1.1/sites/$site/stats/visits` — visits/views over time
- [x] `GET /rest/v1.1/sites/$site/stats/top-authors` — top authors by views
- [x] `GET /rest/v1.1/sites/$site/stats/clicks` — outbound click stats
- [x] `GET /rest/v1.1/sites/$site/stats/file-downloads` — file download stats
- [x] `GET /rest/v1.1/sites/$site/stats/top-posts` — top posts/pages by views
- [x] `GET /rest/v1.1/sites/$site/stats/referrers` — traffic source stats
- [ ] `POST /rest/v1.1/sites/$site/stats/referrers/spam/new` — report referrer as spam
- [ ] `POST /rest/v1.1/sites/$site/stats/referrers/spam/delete` — unreport referrer spam
- [x] `GET /rest/v1.1/sites/$site/stats/search-terms` — search engine terms
- [x] `GET /rest/v1.1/sites/$site/stats/video-plays` — video play stats
- [ ] `GET /rest/v1.1/sites/$site/stats/location-views/country` — views by country
- [x] `GET /rest/v1.1/sites/$site/stats/location-views/region` — views by region
- [x] `GET /rest/v1.1/sites/$site/stats/location-views/city` — views by city
- [ ] `GET /rest/v1.1/sites/$site/stats/archives` — archive stats
- [x] `GET /rest/v1.1/sites/$site/stats/devices/screensize` — device stats by screen size
- [x] `GET /rest/v1.1/sites/$site/stats/devices/platform` — device stats by platform
- [x] `GET /rest/v1.1/sites/$site/stats/devices/browser` — device stats by browser
- [x] `GET /rest/v1.1/sites/$site/stats/utm/$grouping` — UTM stats
- [ ] `GET /rest/v1.1/sites/$site/stats/opens/emails/$post/rate` — email open rate

## Stats — WordAds

- [ ] `GET /rest/v1.1/sites/$site/wordads/stats` — WordAds stats (impressions, revenue)
- [ ] `GET /rest/v1.1/sites/$site/wordads/earnings` — WordAds earnings

## Blaze

- [ ] `GET /wpcom/v2/sites/$site/wordads/dsp/api/v1.1/campaigns` — list Blaze campaigns
- [ ] `POST /wpcom/v2/sites/$site/wordads/dsp/api/v1.1/campaigns` — create Blaze campaign
- [ ] `GET /wpcom/v2/sites/$site/wordads/dsp/api/v1.1/campaigns/objectives` — campaign objectives
- [ ] `GET /wpcom/v2/sites/$site/wordads/dsp/api/v1.1/targeting/locations` — targeting locations
- [ ] `GET /wpcom/v2/sites/$site/wordads/dsp/api/v1.1/targeting/page_topics` — targeting topics
- [ ] `GET /wpcom/v2/sites/$site/wordads/dsp/api/v1.1/targeting/devices` — targeting devices
- [ ] `GET /wpcom/v2/sites/$site/wordads/dsp/api/v1.1/targeting/languages` — targeting languages
- [ ] `POST /wpcom/v2/sites/$site/wordads/dsp/api/v1.1/suggestions` — AI ad suggestions
- [ ] `POST /wpcom/v2/sites/$site/wordads/dsp/api/v1.1/forecast` — impression forecast
- [ ] `GET /wpcom/v2/sites/$site/wordads/dsp/api/v1.1/payment_methods` — payment methods

## Jetpack AI

- [ ] `POST /wpcom/v2/sites/$site/jetpack-openai-query/jwt` — fetch AI JWT token
- [ ] `POST /wpcom/v2/text-completion` — AI text completion (global)
- [ ] `POST /wpcom/v2/sites/$site/jetpack-ai/completions` — AI completions (site-scoped)
- [ ] `POST /wpcom/v2/jetpack-ai-query` — AI message/question query (global)
- [ ] `GET /wpcom/v2/sites/$site/jetpack-ai/ai-assistant-feature` — AI assistant feature status
- [ ] `POST /wpcom/v2/jetpack-ai-transcription` — audio transcription (global)

## Jetpack Tunnel

- [ ] `GET /rest/v1.1/jetpack-blogs/$site/rest-api/` — tunnel WP-API GET requests
- [ ] `POST /rest/v1.1/jetpack-blogs/$site/rest-api/` — tunnel WP-API POST/PUT/PATCH/DELETE
- [ ] `POST /rest/v1/jetpack-install/$site_url` — install Jetpack plugin

## Scan

- [ ] `GET /wpcom/v2/sites/$site/scan` — fetch Jetpack Scan state
- [ ] `POST /wpcom/v2/sites/$site/scan/enqueue` — start scan
- [ ] `GET /wpcom/v2/sites/$site/scan/history` — scan threat history
- [ ] `POST /wpcom/v2/sites/$site/alerts/fix` — fix detected threats
- [ ] `GET /wpcom/v2/sites/$site/alerts/fix` — fetch fix status
- [ ] `POST /wpcom/v2/sites/$site/alerts/$threat_id` — ignore threat

## QR Code Auth

- [ ] `POST /wpcom/v2/auth/qr-code/validate` — validate QR code auth request
- [ ] `POST /wpcom/v2/auth/qr-code/authenticate` — approve QR code login

## Account

- [x] `GET /rest/v1.1/me/` — fetch current user profile
- [ ] `GET /rest/v1.1/me/settings/` — fetch account settings
- [ ] `POST /rest/v1.1/me/settings/` — update account settings
- [ ] `POST /rest/v1.1/me/username/` — change username
- [ ] `GET /rest/v1.1/me/username/validate/$username` — validate username
- [ ] `POST /rest/v1.1/me/sites` — update blogs visibility
- [ ] `POST /rest/v1.1/me/send-verification-email/` — send verification email
- [ ] `GET /rest/v1.1/me/domain-contact-information/` — fetch domain contact info
- [ ] `POST /rest/v1.1/me/domain-contact-information/validate` — validate domain contact info
- [ ] `POST /rest/v1.1/me/account/close/` — close/delete account
- [ ] `GET /rest/v1.1/me/notifications/settings/` — get push notification settings
- [ ] `POST /rest/v1.1/me/notifications/settings/` — save notification settings
- [ ] `POST /rest/v1.1/me/two-step/push-authentication` — send 2FA push auth token
- [ ] `GET /rest/v1.1/me/block/sites/$site/new` — block a blog
- [ ] `POST /rest/v1.1/me/block/sites/$site/delete` — unblock a blog
- [ ] `GET /rest/v1.1/me/keyring-connections` — get keyring connections

## Account — Subscriptions

- [ ] `GET /rest/v1.2/read/following/mine/` — fetch blog subscriptions
- [ ] `POST /rest/v1.2/read/site/$site/comment_email_subscriptions/$action/` — toggle comment email sub
- [ ] `POST /rest/v1.2/read/site/$site/post_email_subscriptions/$action/` — toggle post email sub
- [ ] `POST /rest/v1.2/read/site/$site/post_email_subscriptions/update/` — update email frequency
- [ ] `POST /wpcom/v2/read/sites/$site/notification_subscriptions/$action/` — toggle push notification sub

## Auth & Login

- [x] `POST /oauth2/token` — OAuth2 token (password, 2FA, bearer grants)
- [ ] `POST /rest/v1.3/auth/send-login-email/` — send magic-link login email
- [ ] `POST /rest/v1.1/auth/send-signup-email/` — send magic-link signup email
- [ ] `POST wp-login.php?action=social-login-endpoint` — social provider auth
- [ ] `POST wp-login.php?action=two-step-authentication-endpoint` — 2FA for social login
- [ ] `POST wp-login.php?action=send-sms-code-endpoint` — send 2FA SMS code
- [ ] `POST wp-login.php?action=webauthn-challenge-endpoint` — WebAuthn challenge
- [ ] `POST wp-login.php?action=webauthn-authentication-endpoint` — WebAuthn auth
- [ ] `POST /rest/v1.1/me/social-login/connect/` — connect social account
- [ ] `POST /rest/v1.1/me/social-login/disconnect` — disconnect social account
- [ ] `POST /rest/v1.1/users/social/new/` — create account via social provider

## Account — Signup

- [ ] `POST /rest/v1/users/new/` — create new account (v1)
- [ ] `POST /rest/v1.1/users/new/` — create new account (v1.1)
- [ ] `GET /wpcom/v2/users/username/suggestions/` — username suggestions
- [ ] `GET /is-available/email/` — check email availability (v0)
- [ ] `GET /is-available/username/` — check username availability (v0)
- [ ] `GET /is-available/blog/` — check blog name availability (v0)
- [ ] `GET /rest/v1.1/users/$user/auth-options/` — fetch auth options

## Sites

- [ ] `GET /rest/v1.1/me/sites/` — list user's sites (with features)
- [x] `GET /rest/v1.2/me/sites/` — list user's sites (with filters)
- [ ] `GET /rest/v1.1/me/sites/features/` — fetch plan features for all sites
- [x] `GET /rest/v1.1/sites/$site/` — fetch single site
- [ ] `POST /rest/v1.1/sites/new/` — create new site
- [ ] `POST /rest/v1.1/sites/$site/delete/` — delete site
- [ ] `POST /wpcom/v2/sites/$site/launch/` — launch site
- [ ] `GET /rest/v1.1/sites/$site/post-formats/` — fetch post formats
- [ ] `GET /rest/v1.1/sites/$site/roles/` — fetch user roles
- [ ] `GET /rest/v1.3/sites/$site/plans/` — fetch site plans
- [ ] `POST /rest/v1.1/sites/$site/exports/start/` — export site content
- [ ] `GET /rest/v1.1/sites/$site/settings` — fetch general site settings
- [ ] `POST /rest/v1.1/sites/$site/settings` — update general site settings
- [ ] `POST /rest/v1.1/sites/$site/homepage/` — update homepage settings
- [ ] `POST /rest/v1.1/sites/$site/mobile-quick-start/` — complete Quick Start
- [ ] `GET /rest/v1.1/sites/$site/purchases` — get site purchases
- [ ] `GET /rest/v1.1/sites/$site/follows` — list site followers
- [ ] `GET /rest/v1.1/connect/site-info/` — fetch connection info for URL
- [x] `GET /rest/v1.1/sites/$site_url/` — fetch site by URL (unauthenticated)

## Sites — Editors

- [ ] `GET /wpcom/v2/sites/$site/gutenberg/` — fetch site editors
- [ ] `POST /wpcom/v2/sites/$site/gutenberg/` — set mobile editor
- [ ] `POST /wpcom/v2/me/gutenberg/` — set mobile editor for all sites

## Sites — Block Layouts

- [ ] `GET /wpcom/v2/sites/$site/block-layouts/` — fetch block layouts (WPCom)
- [ ] `GET /wpcom/v2/common-block-layouts/` — fetch block layouts (self-hosted)

## Sites — Automated Transfers

- [ ] `GET /rest/v1.1/sites/$site/automated-transfers/eligibility/` — check eligibility
- [ ] `POST /rest/v1.1/sites/$site/automated-transfers/initiate/` — initiate transfer
- [ ] `GET /rest/v1.1/sites/$site/automated-transfers/status/` — check transfer status

## Sites — Jetpack Features

- [ ] `GET /wpcom/v2/sites/$site/atomic-auth-proxy/read-access-cookies/` — fetch access cookies
- [ ] `GET /wpcom/v2/sites/$site/rewind/capabilities/` — Jetpack Backup capabilities
- [ ] `GET /wpcom/v2/sites/$site/jetpack-social/` — Jetpack Social info
- [ ] `GET /wpcom/v2/sites/$site/hosting/error-logs/` — Atomic PHP error logs
- [ ] `GET /wpcom/v2/sites/$site/hosting/logs/` — Atomic web server logs
- [ ] `GET /rest/v1.1/sites/$site/jetpack/modules` — get Jetpack module settings
- [ ] `POST /rest/v1.1/sites/$site/jetpack/modules/$module` — set Jetpack module
- [ ] `GET /rest/v1.1/jetpack-blogs/$site` — get Jetpack monitor settings
- [ ] `POST /rest/v1.1/jetpack-blogs/$site` — set Jetpack monitor settings
- [ ] `POST /rest/v1.1/jetpack-blogs/$site/mine/delete` — disconnect from Jetpack

## Sites — Cross-Posts

- [ ] `GET /wpcom/v2/sites/$site/xposts/` — fetch cross-post sites

## Domains

- [x] `GET /rest/v1.1/domains/suggestions/` — domain name suggestions
- [x] `GET /rest/v1.3/domains/$domain/is-available/` — check domain availability
- [x] `GET /rest/v1.1/domains/supported-states/$country/` — states for country
- [x] `GET /rest/v1.1/domains/supported-countries/` — supported countries
- [x] `GET /rest/v1.1/all-domains/` — all user's domains
- [x] `GET /rest/v1.1/sites/$site/domains/` — site domains
- [ ] `GET /rest/v1.1/domains/$domain/price/` — domain price
- [ ] `POST /rest/v1.1/sites/$site/domains/primary/` — set primary domain

## Menus

- [ ] `GET /rest/v1.1/sites/$site/menus` — list menus and locations
- [ ] `POST /rest/v1.1/sites/$site/menus/new` — create menu
- [ ] `POST /rest/v1.1/sites/$site/menus/$menu` — update menu
- [ ] `POST /rest/v1.1/sites/$site/menus/$menu/delete` — delete menu

## Dashboard

- [ ] `GET /wpcom/v2/sites/$site/dashboard/cards-data/` — fetch dashboard cards

## Mobile

- [ ] `GET /wpcom/v2/mobile/feature-flags/` — fetch feature flags
- [ ] `POST /wpcom/v2/mobile/migration/` — Jetpack migration complete
- [ ] `GET /wpcom/v2/mobile/remote-config/` — fetch remote config
- [ ] `GET /wpcom/v2/mobile/share-app-link` — get app recommendation template

## Geo

- [ ] `GET /geo/` — IP geolocation country code (v0)

## Verticals

- [x] `GET /wpcom/v2/segments/` — fetch site creation segments
- [ ] `GET /wpcom/v2/verticals` — search site verticals
- [ ] `GET /wpcom/v2/verticals/prompt` — verticals prompt text

## Timezones

- [ ] `GET /wpcom/v2/timezones` — fetch timezone list

## Reader — Posts

- [ ] `GET /rest/v1.2/read/sites/$site/posts/` — fetch posts for a blog
- [ ] `GET /rest/v1.2/read/feed/$feed/posts/` — fetch posts for a feed
- [ ] `GET /rest/v1.2/read/tags/$tag/posts` — fetch posts for a tag
- [ ] `GET /rest/v1.2/read/search` — search reader posts
- [ ] `GET /rest/v1.2/read/sites/$site/posts/$post_id` — fetch single reader post (v1.2)
- [ ] `GET /rest/v1.1/read/sites/$site/posts/$post_id` — fetch single reader post (v1.1)
- [ ] `GET /rest/v1.3/read/feed/$feed/posts/$post_id` — fetch feed post
- [ ] `GET /rest/v1.1/sites/$site/posts/slug:$slug` — fetch post by slug
- [ ] `GET /rest/v1.2/read/site/$site/post/$post_id/related` — related posts
- [ ] `POST /rest/v1.1/sites/$site/posts/$post_id/likes/new` — like a post
- [ ] `POST /rest/v1.1/sites/$site/posts/$post_id/likes/mine/delete` — unlike a post

## Reader — Blogs & Feeds

- [ ] `POST /rest/v1.1/sites/$site/follows/new` — follow a blog
- [ ] `POST /rest/v1.1/sites/$site/follows/mine/delete` — unfollow a blog
- [ ] `POST /rest/v1.1/read/following/mine/new` — follow a feed by URL
- [ ] `POST /rest/v1.1/read/following/mine/delete` — unfollow a feed by URL
- [ ] `GET /rest/v1.1/read/sites/$site` — get blog info
- [ ] `GET /rest/v1.1/read/feed/$feed` — get feed info
- [ ] `GET /rest/v1.2/read/following/mine` — list followed blogs (paginated)
- [ ] `GET /rest/v1.3/read/menu` — get Reader menu (tags)
- [ ] `GET /wpcom/v2/read/interests` — fetch interest/onboarding tags
- [ ] `GET /wpcom/v2/read/streams/discover` — Discover recommended posts
- [ ] `GET /wpcom/v2/read/streams/$stream` — named discover streams (generic)
- [ ] `GET /wpcom/v2/read/tags/cards` — reader cards for tags
- [ ] `GET /wpcom/v2/read/tags/posts` — Discover latest posts

## Reader — Tags

- [ ] `GET /rest/v1.2/read/tags/$slug` — fetch tag info by slug
- [ ] `POST /rest/v1.1/read/tags/$tag/mine/new` — follow a tag
- [ ] `POST /rest/v1.1/read/tags/$tag/mine/delete` — delete followed tag
- [ ] `POST /rest/v1.2/read/tags/mine/new` — add followed tags (batch)

## Reader — Comments

- [ ] `GET /rest/v1.1/sites/$site/posts/$post_id/replies/` — fetch post comments (paginated)

## Reader — Post Subscribers

- [ ] `GET /rest/v1.1/sites/$site/posts/$post_id/subscribers/mine` — get my subscription
- [ ] `POST /rest/v1.1/sites/$site/posts/$post_id/subscribers/new` — subscribe to post
- [ ] `POST /rest/v1.1/sites/$site/posts/$post_id/subscribers/mine/delete` — unsubscribe
- [ ] `POST /rest/v1.1/sites/$site/posts/$post_id/subscribers/mine/update` — toggle push

## Reader — Seen Status

- [ ] `POST /wpcom/v2/seen-posts/seen/new` — mark feed post as seen
- [ ] `POST /wpcom/v2/seen-posts/seen/blog/new` — mark blog post as seen
- [ ] `POST /wpcom/v2/seen-posts/seen/delete` — mark feed post as unseen
- [ ] `POST /wpcom/v2/seen-posts/seen/blog/delete` — mark blog post as unseen

## Reader — URL Resolution

- [ ] `GET /wpcom/v2/mobile/resolve-reader-url` — resolve URL to reader post/site

## Reader — Following Status

- [ ] `GET /rest/v1.1/sites/$site/follows/mine` — check if following site

## Reader — Tracking

- [ ] `GET pixel.wp.com/g.gif` — page view tracking pixel

## Notifications

- [ ] `GET /rest/v1.1/notifications` — list notifications
- [ ] `GET /rest/v1.1/notifications/$note_id` — fetch single notification
- [ ] `POST /rest/v1.1/notifications/seen` — mark notifications as seen
- [ ] `POST /rest/v1.1/notifications/read` — mark notification as read
- [ ] `POST /rest/v1/devices/new` — register device for push
- [ ] `POST /rest/v1/devices/$device_id/delete` — unregister device

## People & Invites

- [ ] `GET /rest/v1.1/sites/$site/users` — list site users
- [ ] `POST /rest/v1.1/sites/$site/users/$user_id` — update user role
- [ ] `POST /rest/v1.1/sites/$site/users/$user_id/delete` — remove user
- [ ] `GET /rest/v1.1/sites/$site/viewers` — list viewers (private sites)
- [ ] `POST /rest/v1.1/sites/$site/viewers/$viewer_id/delete` — remove viewer
- [x] `POST /rest/v1.1/sites/$site/followers/$follower_id/delete` — remove follower
- [x] `POST /rest/v1.1/sites/$site/email-followers/$follower_id/delete` — remove email follower
- [ ] `POST /rest/v1.1/sites/$site/invites/validate` — validate invite usernames
- [ ] `POST /rest/v1.1/sites/$site/invites/new` — send invitations
- [ ] `GET /rest/v1.1/sites/$site/invites` — get invite link status
- [ ] `POST /wpcom/v2/sites/$site/invites/links/generate` — generate invite links
- [ ] `POST /wpcom/v2/sites/$site/invites/links/disable` — disable invite links
- [ ] `GET /rest/v1.1/batch/` — batch user lookups

## Subscribers Management

- [x] `GET /wpcom/v2/sites/$site/subscribers` — list subscribers
- [x] `GET /wpcom/v2/sites/$site/subscribers/individual` — subscriber details
- [x] `GET /wpcom/v2/sites/$site/individual-subscriber-stats` — subscriber stats
- [x] `POST /wpcom/v2/sites/$site/subscribers/import` — import subscribers by email

## Publicize (Social Sharing)

- [ ] `GET /rest/v1.1/meta/external-services` — list publicize services (global)
- [ ] `GET /rest/v1.1/sites/$site/publicize-connections` — list connections
- [ ] `POST /rest/v1.1/sites/$site/publicize-connections/new` — create connection
- [ ] `POST /rest/v1.1/sites/$site/publicize-connections/$conn_id` — update connection
- [ ] `POST /rest/v1.1/sites/$site/publicize-connections/$conn_id/delete` — disconnect
- [ ] `GET /wpcom/v2/sites/$site/external-services?type=publicize` — list services
- [ ] `GET /rest/v1.1/sites/$site/sharing-buttons` — get sharing buttons
- [ ] `POST /rest/v1.1/sites/$site/sharing-buttons` — set sharing buttons

## Suggestions

- [ ] `GET /rest/v1/users/suggest` — fetch username suggestions
- [ ] `GET /rest/v1/sites/$site/tags` — fetch site tags for suggestions

## Media

- [ ] `GET /rest/v1.1/sites/$site/media` — list media items
- [ ] `GET /rest/v1.1/sites/$site/media/$media_id` — fetch single media item
- [ ] `POST /rest/v1.1/sites/$site/media/$media_id` — update media metadata
- [ ] `POST /rest/v1.1/sites/$site/media/new` — upload media file
- [ ] `POST /rest/v1.1/sites/$site/media/$media_id/delete` — delete media item
- [ ] `POST /rest/v1.1/sites/$site/external-media-upload` — upload stock media by URL
- [ ] `GET /rest/v1.1/videos/$videopress_id` — VideoPress metadata
- [ ] `POST /rest/v2.0/sites/$site/media/videopress-playback-jwt/$videopress_id` — VideoPress JWT

## Stock Media

- [ ] `GET /rest/v1.1/meta/external-media/pexels` — search Pexels stock media

## Plugins

- [ ] `GET /rest/v1.2/sites/$site/plugins` — list installed plugins
- [ ] `POST /rest/v1.2/sites/$site/plugins/$plugin` — configure plugin (active/auto-update)
- [ ] `POST /rest/v1.2/sites/$site/plugins/$plugin/delete` — delete plugin
- [ ] `POST /rest/v1.2/sites/$site/plugins/$plugin/install` — install plugin
- [ ] `POST /rest/v1.2/sites/$site/plugins/$plugin/update` — update plugin
- [ ] `GET /wpcom/v2/plugins/featured` — featured plugins list

## Plans & Products

- [ ] `GET /rest/v1.5/plans` — list all WPCom plans
- [ ] `GET /wpcom/v2/plans/mobile` — fetch mobile plan offers
- [x] `GET /rest/v1.1/products` — list products (with optional type filter)

## Transactions

- [ ] `GET /rest/v1.1/me/transactions/supported-countries` — supported countries
- [x] `POST /rest/v1.1/me/shopping-cart/$site` — create shopping cart (with site)
- [x] `POST /rest/v1.1/me/shopping-cart/no-site` — create shopping cart (no site)
- [ ] `POST /rest/v1.1/me/transactions` — redeem cart using credits

## Mobile Pay

- [ ] `POST /wpcom/v2/iap/orders` — create in-app purchase order

## Themes

- [ ] `GET /rest/v1.2/themes` — list WPCom themes
- [ ] `GET /rest/v1/sites/$site/themes` — list installed themes (Jetpack)
- [ ] `GET /rest/v1.1/sites/$site/themes/mine` — fetch current theme
- [ ] `POST /rest/v1.1/sites/$site/themes/mine` — activate theme
- [ ] `POST /rest/v1.1/sites/$site/themes/$theme/install` — install theme
- [ ] `POST /rest/v1.1/sites/$site/themes/$theme/delete` — delete theme
- [ ] `GET /wpcom/v2/common-starter-site-designs` — fetch starter designs
- [ ] `GET /rest/v1.1/themes/$theme` — fetch single theme by ID
- [ ] `GET /wpcom/v2/themes` — list WPCom themes (v2)
- [ ] `GET /rest/v1.1/sites/$site/themes/purchased` — purchased themes

## Activity Log

- [ ] `GET /wpcom/v2/sites/$site/activity/` — fetch activity log (paginated)
- [ ] `GET /wpcom/v2/sites/$site/activity/rewindable` — rewindable activity only
- [ ] `GET /wpcom/v2/sites/$site/activity/count/group/` — activity type counts
- [ ] `GET /wpcom/v2/sites/$site/rewind/` — fetch rewind/backup status
- [ ] `POST /rest/v1/activity-log/$site/rewind/to/$rewind_id/` — rewind to point
- [ ] `POST /wpcom/v2/sites/$site/rewind/downloads/` — initiate backup download
- [ ] `GET /wpcom/v2/sites/$site/rewind/downloads/` — fetch backup download status
- [ ] `POST /wpcom/v2/sites/$site/rewind/downloads/$download_id/` — dismiss backup download

## What's New

- [ ] `GET /wpcom/v2/mobile/feature-announcements/` — fetch feature announcements

## Reader — Search

- [ ] `GET /rest/v1.1/read/feed/` — search reader sites/feeds
- [ ] `GET /rest/v1.2/freshly-pressed` — freshly pressed posts

