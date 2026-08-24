# Conventions

Rules for modelling the WordPress and WordPress.com REST APIs in this crate, so the same decisions don't get re-argued on every endpoint.

## Types

A type is a promise. Two values share one only when they are the same thing — not nearly, not for now. If you can't say yes without a qualifier, the answer is no. "Same shape today", "same apart from one endpoint", "same unless the server does X" are all no.

Share helpers where they genuinely overlap; that's what the overlap is good for.

## Dates

When you model a point in time, reach for one of these two types rather than a bare `String`.

**`WpGmtDateTime`** — the value resolves to an absolute instant, because it is UTC or carries an offset. The bindings lower it to a unix timestamp, so a value that isn't genuinely an instant becomes a wrong one.

**`WpDateString`** — it denotes a date but can't be resolved to an instant: a bare calendar date (`2026-08-06`), or a datetime in the *site's* timezone (`2026-08-06 09:15:49`), which needs the site's offset to place.

Where an endpoint sends both forms of the same timestamp, model both — the GMT one as `WpGmtDateTime`, its local twin as `WpDateString`.

Every way of reading a `WpGmtDateTime` accepts the same set: an offset-bearing value, the offsetless WordPress form, MySQL's, and a unix timestamp. The offsetless forms are read as UTC, so only reach for this type once you know the value is GMT.

Decide from what the endpoint implementation produces, not from the field's name or the schema's wording. Both directions bite: a "most active day" is an instant, because it comes off a comment's GMT timestamp, while a "last updated" can be prose for display.

Query parameters are no different. `/wp/v2`'s `after` is documented as ISO-8601 and matched against the site-local `post_date` column, but `WP_Date_Query` converts an offset-bearing value into the site's timezone first, so `WpGmtDateTime` is right for it.

### Absent and unparseable values

`wp_com`'s domain fields send boolean `false` rather than `null` when a date doesn't apply; `deserialize_optional_date_string` covers that for `WpDateString`. `deserialize_optional_wp_gmt_date_time` treats `null` and `""` as absent.

WordPress's zero date — `0000-00-00 00:00:00`, and what PHP's formatters make of it — is not a datetime. `deserialize_optional_wp_gmt_date_time` reads it as `None`; everywhere else it is an error, because the alternative is an instant in 1 BCE that looks like data.

So a field on an endpoint that doesn't guard the column has to be an `Option` — otherwise one such record fails the entire response. `/wp/v2` guards posts, but not users or comments.
