# Recorded Cloudflare responses

Raw HTTP/1.1 responses (status line, headers, blank line, body) used to test how `wordpress-rs` recognises errors that Cloudflare generates itself. The tests replay them through a local socket (`wp_api_integration_tests/tests/test_cloudflare_errors.rs`) and parse them directly (`wp_api/src/cloudflare.rs`).

Files starting `cf-` are Cloudflare's own responses. Files starting `origin-` were written by the site's server and must **not** be classified as Cloudflare's, even when they came through Cloudflare.

## Where they came from

Recorded on 2026-10-26 and 2026-10-27 with `Accept: application/json`, like the library's requests.

**Real failures, from a test zone** (`wpmtcf.co`, Free plan) set up to produce each one:

| Files | How the zone produces it |
|---|---|
| `cf-413-html.http` | A 150 MB upload to `testing.` (the Free plan's limit is 100 MB) |
| `cf-502-html.http` | The same upload. Cloudflare sometimes answers 502 instead, once the whole body is sent |
| `cf-520-json.http` | `cf-520.`: plain HTTP to the server's SSH port |
| `cf-521-json.http` | `cf-521.`: a server that refuses the connection |
| `cf-522-json.http`, `cf-522-plain-text.http` | `cf-522.`: a port whose firewall drops packets |
| `cf-525-json.http` | `cf-525.`: TLS to the server's SSH port |
| `cf-526-json.http` | `cf-526.`: "Full (strict)" SSL against a certificate for another name |
| `cf-1002-json.http` | `cf-523.`: a record pointing at a reserved IP address |
| `cf-1016-json.http` | `cf-1016.`: a CNAME to a hostname that doesn't exist |
| `cf-1015-json.http` | A rate-limiting rule on `cf-rules./cf-test/ratelimit` |
| `cf-waf-block-page.http` | A WAF custom rule that blocks `cf-rules./cf-test/block`. **HTML, even when JSON is asked for** |
| `cf-challenge.http` | A managed-challenge rule on `cf-rules./cf-test/challenge` (`cf-mitigated: challenge`). The JS-challenge (`/cf-test/js-challenge`) and interactive-challenge (`/cf-test/interactive`) rules send the same kind of page |
| `cf-522-html.http`, `cf-1016-html.http`, `cf-1002-html.http` | The branded HTML pages for `cf-522.`, `cf-1016.` and `cf-523.`, requested with `Accept: */*` as the login flow does |
| `origin-wp-error-behind-cloudflare.http` | WordPress's own 401 from `testing.`, through Cloudflare |

**Cloudflare's preview of a response**, `https://developers.cloudflare.com/cdn-cgi/error/<code>`, for codes the zone can't easily produce: `cf-500-json.http`, `cf-502-json.http`, `cf-504-json.http`, `cf-523-json.http`, `cf-524-json.http`, `cf-1020-json.http`, `cf-1020-plain-text.http` and `cf-1010-plain-text.http` (from the test zone's own `/cdn-cgi/error/1010`). The real 520–526 responses match their previews' shape.

**Elsewhere:**

- `cf-1003-plain-text.http`: `http://104.21.20.53/` (direct IP access).
- `origin-nginx-403-behind-cloudflare.http`, `origin-plain-forbidden-behind-cloudflare.http`: real sites' own 403s, through Cloudflare.
- `origin-apache-502-behind-cloudflare.http`, `origin-basic-auth-401-behind-cloudflare.http`, `origin-nginx-413.http`: stock Apache and nginx error pages, written by hand.

## The test zone

The live tests (`--ignored` in `test_cloudflare_errors.rs`, `CloudflareLiveTests.swift`, `CloudflareLiveTest.kt`) run against it when `WPRS_CLOUDFLARE_TEST_ZONE=wpmtcf.co` is set. Everything the setup added is tagged `wprs-cf-test` in the zone: DNS record comments, and rule descriptions in the origin, configuration, WAF custom and rate-limiting rule phases.

What a Free-plan zone can't do, so these tests can't cover it either: custom WAF block responses ("not entitled to use a custom response") and Custom Error rules (a limit of 0). So on Free, a site owner can't replace Cloudflare's error pages.

## Cloudflare only sends JSON when asked with `Accept-Encoding`

Cloudflare's [RFC 9457 JSON errors](https://developers.cloudflare.com/fundamentals/reference/error-responses/) need `Accept: application/json` **and** an `Accept-Encoding` header (any value). Without the latter you get `error code: NNNN` as plain text. `curl` sends no `Accept-Encoding` by default, which is why the JSON can look absent; `reqwest`, `URLSession` and OkHttp all send it. Direct IP access (1003) has no zone and is plain text either way.

## Sanitizing

Hostnames of third-party sites are replaced with `example.com` and the requester's IP with `192.0.2.1`. Per-request tokens in the challenge page are replaced with `REDACTED`. Headers irrelevant to error detection (cookies, CSP, reporting endpoints, caching, `Content-Length`) are dropped; the replay server adds its own `Content-Length`.
