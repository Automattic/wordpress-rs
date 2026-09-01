# `api-details` fixtures

Sample WordPress REST API index responses (`GET /wp-json/`), used to test `WpApiDetails` parsing.

Each `test-case-NN.json` is loaded by the `test_json` helper in `wp_api/src/login.rs` and fed through `test_api_details_json`, a smoke test asserting the body deserializes into `WpApiDetails`. Some cases also drive targeted tests, noted below.

These fixtures deliberately cover the shapes WordPress emits in the wild — including the PHP-isms a naive JSON model gets wrong. The recurring ones:

- **`authentication` and `routes` as `[]` vs `{}`** — PHP's `json_encode([])` renders an empty map as `[]`, not `{}`. An empty `authentication` (or `routes`) therefore arrives as an array. `WpApiDetails.authentication` tolerates both via `deserialize_empty_array_or_hashmap`; `routes` is a strict object.
- **`gmt_offset` as a string or a number** — real sites emit both `"0"` and `0`.
- **`site_icon_url` as `false`, a string, or absent** — handled by `deserialize_false_or_string`.
- **A UTF-8 BOM** prefixing the body — stripped in `WpApiDetails::try_from`.

## Catalogue

| File | Size | What it exercises |
| --- | --- | --- |
| `test-case-01.json` | 340 B | UTF-8 **BOM** prefix; `authentication: []` (empty-array form); `site_icon_url: false`; `gmt_offset: -3` (number). Covers BOM stripping, the `false` site-icon case, and empty-auth-as-array. |
| `test-case-02.json` | 906 B | `site_icon_url` field **absent**; `authentication: []`; carries a `_links` block. Covers the missing-optional-field path. |
| `test-case-03.json` | 788 KB | Real self-hosted Jetpack dump (`jetpack.wpmt.co`); `authentication: {application-passwords}`; `gmt_offset: "0"` (**string**); 19 namespaces, 498 routes. Also drives `test_has_namespace`, `test_route_args`, `test_has_route`, and `test_has_route_for_endpoint_with_wp_org_fixture`. |
| `test-case-04.json` | 1.9 MB | Real WordPress.com dump (`Mobile.blog`); `authentication: []` — an **empty array from a real, live site**, proving core itself emits `[]`; 18 namespaces, 1632 routes. |
| `test-case-05.json` | 2 KB | Compact fixture (`Example WordPress Site`); `authentication: {application-passwords}`; `gmt_offset: 13`; 20 namespaces but a single route. |
| `test-case-06.json` | 1.4 MB | Real WordPress.com dump; `authentication: {oauth2}` — the **OAuth2 scheme**; 21 namespaces, 1384 routes. |
| `test-case-07.json` | 344 KB | Real dump (`oauth-testing`); `authentication: {application-passwords, oauth2}` — **both schemes at once**; `gmt_offset: "0"` (string); 5 namespaces, 127 routes. |
| `test-case-08.json` | 391 B | **Minimal API root a private site advertises.** Only `application-passwords` authentication; empty `namespaces` and `routes` (`{}`); `gmt_offset: 0`; no `timezone_string` or `site_icon_url`. Mirrors what a private site emits so a client can still discover the application-password login endpoint. Also drives `test_parse_private_site_api_root`. |

## Adding a fixture

Add the `test-case-NN.json` file, a `#[case(...)]` line to `test_api_details_json` in `wp_api/src/login.rs`, and a row here describing what the new case covers.
